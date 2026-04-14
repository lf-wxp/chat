//! Discovery state manager for handling invitations, peers, and SDP negotiations.

use std::collections::HashSet;

use dashmap::DashMap;
use message::RoomId;
use message::UserId;
use message::signaling::{ConnectionInvite, MultiInvite};
use tracing::{debug, info};

use super::rate_limit::UserRateLimit;
use super::types::{
  InvitationError, InvitationId, MAX_UNANSWERED_INVITATIONS_PER_TARGET, MultiInviteAcceptResult,
  MultiInviteState, MultiInviteStats, PendingInvitation, SdpNegotiationState,
};

// =============================================================================
// Discovery State Manager
// =============================================================================

/// Global state for user discovery and invitation management.
#[derive(Debug)]
pub struct DiscoveryState {
  /// Pending invitations: (from_user_id, to_user_id) -> invitation.
  pending_invitations: DashMap<(UserId, UserId), PendingInvitation>,
  /// Unanswered invitation count per target user.
  unanswered_counts: DashMap<UserId, usize>,
  /// Rate limit trackers per user.
  rate_limits: DashMap<UserId, UserRateLimit>,
  /// Multi-user invitation tracking: invitation_id -> (room_id, accepted_users, declined_users).
  multi_invites: DashMap<InvitationId, MultiInviteState>,
  /// Active peer connections: user_id -> set of connected peer user_ids.
  active_peers: DashMap<UserId, HashSet<UserId>>,
  /// SDP negotiation state: (from_user_id, to_user_id) -> negotiation state.
  sdp_negotiations: DashMap<(UserId, UserId), SdpNegotiationState>,
}

impl DiscoveryState {
  /// Create a new discovery state.
  #[must_use]
  pub fn new() -> Self {
    Self {
      pending_invitations: DashMap::new(),
      unanswered_counts: DashMap::new(),
      rate_limits: DashMap::new(),
      multi_invites: DashMap::new(),
      active_peers: DashMap::new(),
      sdp_negotiations: DashMap::new(),
    }
  }

  // ===========================================================================
  // Rate Limiting
  // ===========================================================================

  /// Check if a user can send an invitation (rate limiting).
  pub fn can_send_invitation(&self, from: &UserId) -> bool {
    let mut rate_limit = self.rate_limits.entry(from.clone()).or_default();
    rate_limit.can_send()
  }

  /// Get remaining invitation quota for a user.
  #[must_use]
  pub fn get_remaining_quota(&self, from: &UserId) -> (usize, usize) {
    let rate_limit = self.rate_limits.entry(from.clone()).or_default();
    (
      rate_limit.remaining_this_minute(),
      rate_limit.remaining_this_hour(),
    )
  }

  // ===========================================================================
  // Invitation Management
  // ===========================================================================

  /// Send a connection invitation.
  /// Returns Ok(invitation_id) if successful, Err(reason) if blocked.
  pub fn send_invitation(
    &self,
    invite: &ConnectionInvite,
  ) -> Result<InvitationId, InvitationError> {
    // Check rate limit
    if !self.can_send_invitation(&invite.from) {
      return Err(InvitationError::RateLimitExceeded);
    }

    // Check if there's already a pending invitation from this user to the target
    if self
      .pending_invitations
      .contains_key(&(invite.from.clone(), invite.to.clone()))
    {
      return Err(InvitationError::AlreadyPending);
    }

    // Check unanswered count for target
    {
      let mut unanswered = self.unanswered_counts.entry(invite.to.clone()).or_insert(0);
      if *unanswered >= MAX_UNANSWERED_INVITATIONS_PER_TARGET {
        return Err(InvitationError::TargetLimitExceeded);
      }
      // Increment the count while still holding the lock
      *unanswered += 1;
    }

    // Record the invitation
    let pending =
      PendingInvitation::new(invite.from.clone(), invite.to.clone(), invite.note.clone());
    let id = pending.id.clone();

    self
      .pending_invitations
      .insert((invite.from.clone(), invite.to.clone()), pending);

    // Record rate limit
    {
      let mut rate_limit = self.rate_limits.entry(invite.from.clone()).or_default();
      rate_limit.record_invitation();
    }

    debug!(
      from = %invite.from,
      to = %invite.to,
      invitation_id = %id,
      "Connection invitation sent"
    );

    Ok(id)
  }

  /// Check for bidirectional invitation conflict.
  /// Returns Some(invitation) if a reverse invitation exists.
  #[must_use]
  pub fn check_bidirectional_conflict(
    &self,
    from: &UserId,
    to: &UserId,
  ) -> Option<PendingInvitation> {
    // Check if target has a pending invitation to the sender
    self
      .pending_invitations
      .get(&(to.clone(), from.clone()))
      .map(|entry| entry.clone())
  }

  /// Merge bidirectional invitations (auto-accept).
  /// Removes both invitations and returns them.
  pub fn merge_bidirectional_invitations(
    &self,
    from: &UserId,
    to: &UserId,
  ) -> Option<(PendingInvitation, PendingInvitation)> {
    // Get both invitations
    let invite1 = self.pending_invitations.remove(&(from.clone(), to.clone()));
    let invite2 = self.pending_invitations.remove(&(to.clone(), from.clone()));

    match (invite1, invite2) {
      (Some((_, i1)), Some((_, i2))) => {
        // Decrement unanswered counts
        if let Some(mut count) = self.unanswered_counts.get_mut(from) {
          *count = count.saturating_sub(1);
        }
        if let Some(mut count) = self.unanswered_counts.get_mut(to) {
          *count = count.saturating_sub(1);
        }

        info!(
          user1 = %from,
          user2 = %to,
          "Merged bidirectional invitations"
        );

        Some((i1, i2))
      }
      _ => None,
    }
  }

  /// Accept an invitation.
  /// Returns the accepted invitation if found.
  pub fn accept_invitation(&self, from: &UserId, to: &UserId) -> Option<PendingInvitation> {
    if let Some((_, invitation)) = self.pending_invitations.remove(&(from.clone(), to.clone())) {
      // Decrement unanswered count
      if let Some(mut count) = self.unanswered_counts.get_mut(to) {
        *count = count.saturating_sub(1);
      }

      debug!(
        from = %from,
        to = %to,
        invitation_id = %invitation.id,
        "Invitation accepted"
      );

      Some(invitation)
    } else {
      None
    }
  }

  /// Decline an invitation.
  /// Returns the declined invitation if found.
  pub fn decline_invitation(&self, from: &UserId, to: &UserId) -> Option<PendingInvitation> {
    if let Some((_, invitation)) = self.pending_invitations.remove(&(from.clone(), to.clone())) {
      // Decrement unanswered count
      if let Some(mut count) = self.unanswered_counts.get_mut(to) {
        *count = count.saturating_sub(1);
      }

      debug!(
        from = %from,
        to = %to,
        invitation_id = %invitation.id,
        "Invitation declined"
      );

      Some(invitation)
    } else {
      None
    }
  }

  /// Get timed out invitations.
  /// Returns a list of timed out invitations and removes them from pending.
  pub fn get_timed_out_invitations(&self) -> Vec<PendingInvitation> {
    let mut timed_out = Vec::new();

    // Collect keys to remove
    let keys_to_remove: Vec<(UserId, UserId)> = self
      .pending_invitations
      .iter()
      .filter(|entry| entry.value().is_timed_out())
      .map(|entry| entry.key().clone())
      .collect();

    // Remove and collect
    for key in keys_to_remove {
      if let Some((_, invitation)) = self.pending_invitations.remove(&key) {
        // Decrement unanswered count
        if let Some(mut count) = self.unanswered_counts.get_mut(&key.1) {
          *count = count.saturating_sub(1);
        }

        timed_out.push(invitation);
      }
    }

    timed_out
  }

  /// Check if a specific pending invitation exists.
  #[must_use]
  pub fn has_pending_invitation(&self, from: &UserId, to: &UserId) -> bool {
    self
      .pending_invitations
      .contains_key(&(from.clone(), to.clone()))
  }

  /// Get pending invitation count for a user.
  #[must_use]
  pub fn pending_invitation_count(&self, user_id: &UserId) -> usize {
    self
      .pending_invitations
      .iter()
      .filter(|entry| entry.key().0 == *user_id || entry.key().1 == *user_id)
      .count()
  }

  /// Get all pending invitations sent by a user.
  #[must_use]
  pub fn get_pending_sent(&self, from: &UserId) -> Vec<PendingInvitation> {
    self
      .pending_invitations
      .iter()
      .filter(|entry| entry.key().0 == *from)
      .map(|entry| entry.value().clone())
      .collect()
  }

  /// Get all pending invitations received by a user.
  #[must_use]
  pub fn get_pending_received(&self, to: &UserId) -> Vec<PendingInvitation> {
    self
      .pending_invitations
      .iter()
      .filter(|entry| entry.key().1 == *to)
      .map(|entry| entry.value().clone())
      .collect()
  }

  /// Clear all pending invitations involving a user (e.g., on disconnect).
  /// Returns the list of removed invitations for notification purposes.
  pub fn clear_pending_invitations_for_user(
    &self,
    user_id: &UserId,
  ) -> Vec<((UserId, UserId), PendingInvitation)> {
    let keys_to_remove: Vec<(UserId, UserId)> = self
      .pending_invitations
      .iter()
      .filter(|entry| {
        let (from, to) = entry.key();
        from == user_id || to == user_id
      })
      .map(|entry| entry.key().clone())
      .collect();

    let mut removed = Vec::with_capacity(keys_to_remove.len());
    for key in keys_to_remove {
      if let Some((k, invitation)) = self.pending_invitations.remove(&key) {
        // Decrement unanswered count for the target
        if let Some(mut count) = self.unanswered_counts.get_mut(&k.1) {
          *count = count.saturating_sub(1);
        }
        debug!(
          from = %k.0,
          to = %k.1,
          "Removed pending invitation for disconnected user"
        );
        removed.push((k, invitation));
      }
    }

    removed
  }

  // ===========================================================================
  // Multi-Invite Management
  // ===========================================================================

  /// Send a multi-user invitation.
  /// Returns Ok(invitation_id) if successful.
  pub fn send_multi_invitation(
    &self,
    invite: &MultiInvite,
  ) -> Result<InvitationId, InvitationError> {
    // Check rate limit
    if !self.can_send_invitation(&invite.from) {
      return Err(InvitationError::RateLimitExceeded);
    }

    // Create pending invitations for each target
    let invitation_id = InvitationId::new();
    let mut valid_targets = Vec::new();

    for target in &invite.targets {
      // Skip if already has pending invitation
      if self
        .pending_invitations
        .contains_key(&(invite.from.clone(), target.clone()))
      {
        continue;
      }

      // Check unanswered count
      let unanswered = self.unanswered_counts.entry(target.clone()).or_insert(0);
      if *unanswered >= MAX_UNANSWERED_INVITATIONS_PER_TARGET {
        continue;
      }

      valid_targets.push(target.clone());
    }

    if valid_targets.is_empty() {
      return Err(InvitationError::NoValidTargets);
    }

    // Create multi-invite state
    let multi_state = MultiInviteState::new(invite.from.clone(), valid_targets.clone());

    self
      .multi_invites
      .insert(invitation_id.clone(), multi_state);

    // Create pending invitations for each valid target
    for target in &valid_targets {
      let pending = PendingInvitation::new(invite.from.clone(), target.clone(), None);
      self
        .pending_invitations
        .insert((invite.from.clone(), target.clone()), pending);

      // Update unanswered count
      *self.unanswered_counts.get_mut(target).unwrap() += 1;
    }

    // Record rate limit (count as one invitation regardless of targets count)
    if let Some(mut rate_limit) = self.rate_limits.get_mut(&invite.from) {
      rate_limit.record_invitation();
    }

    debug!(
      from = %invite.from,
      targets_count = valid_targets.len(),
      invitation_id = %invitation_id,
      "Multi-user invitation sent"
    );

    Ok(invitation_id)
  }

  /// Accept a multi-user invitation.
  /// Returns the room_id if one has been created, or None if this is the first acceptance.
  pub fn accept_multi_invitation(
    &self,
    from: &UserId,
    to: &UserId,
    room_id: RoomId,
  ) -> Option<MultiInviteAcceptResult> {
    // Find the multi-invite that contains this invitation
    for mut entry in self.multi_invites.iter_mut() {
      let multi_state = entry.value_mut();

      // Check if this invitation belongs to this multi-invite
      if multi_state.from == *from && multi_state.targets.contains(to) {
        // Remove from pending invitations
        self.pending_invitations.remove(&(from.clone(), to.clone()));

        // Decrement unanswered count
        if let Some(mut count) = self.unanswered_counts.get_mut(to) {
          *count = count.saturating_sub(1);
        }

        // Add to accepted list if not already
        if !multi_state.accepted.contains(to) {
          multi_state.accepted.push(to.clone());
        }

        // If this is the first acceptance, create the room
        let result = if multi_state.room_id.is_none() {
          multi_state.room_id = Some(room_id.clone());
          MultiInviteAcceptResult::FirstAcceptance {
            room_id,
            remaining_targets: multi_state.remaining_targets(),
          }
        } else {
          MultiInviteAcceptResult::JoinRoom {
            room_id: multi_state.room_id.clone().unwrap(),
          }
        };

        return Some(result);
      }
    }

    None
  }

  /// Decline a multi-user invitation.
  pub fn decline_multi_invitation(&self, from: &UserId, to: &UserId) {
    // Remove from pending invitations
    self.pending_invitations.remove(&(from.clone(), to.clone()));

    // Decrement unanswered count
    if let Some(mut count) = self.unanswered_counts.get_mut(to) {
      *count = count.saturating_sub(1);
    }

    // Update multi-invite state
    for mut entry in self.multi_invites.iter_mut() {
      let multi_state = entry.value_mut();
      if multi_state.from == *from && multi_state.targets.contains(to) {
        if !multi_state.declined.contains(to) {
          multi_state.declined.push(to.clone());
        }
        break;
      }
    }
  }

  /// Check if all targets have responded to a multi-invite.
  /// Returns true if all have accepted, declined, or timed out.
  #[must_use]
  pub fn is_multi_invite_complete(&self, invitation_id: &InvitationId) -> bool {
    if let Some(multi_state) = self.multi_invites.get(invitation_id) {
      multi_state.is_complete()
    } else {
      true
    }
  }

  /// Get multi-invite statistics.
  #[must_use]
  pub fn get_multi_invite_stats(&self, invitation_id: &InvitationId) -> Option<MultiInviteStats> {
    self.multi_invites.get(invitation_id).map(|entry| {
      let state = entry.value();
      MultiInviteStats {
        from: state.from.clone(),
        total_targets: state.targets.len(),
        accepted: state.accepted.len(),
        declined: state.declined.len(),
        room_id: state.room_id.clone(),
      }
    })
  }

  /// Clean up expired multi-invites.
  pub fn cleanup_expired_multi_invites(&self) {
    let keys_to_remove: Vec<InvitationId> = self
      .multi_invites
      .iter()
      .filter(|entry| entry.value().is_timed_out())
      .map(|entry| entry.key().clone())
      .collect();

    for key in keys_to_remove {
      self.multi_invites.remove(&key);
    }
  }

  // ===========================================================================
  // Active Peers Management
  // ===========================================================================

  /// Add an active peer relationship.
  /// This should be called when a PeerEstablished message is sent/received.
  pub fn add_active_peer(&self, user_id: &UserId, peer_id: &UserId) {
    // Add peer to user's peer set
    self
      .active_peers
      .entry(user_id.clone())
      .or_default()
      .insert(peer_id.clone());

    // Also add the reverse relationship
    self
      .active_peers
      .entry(peer_id.clone())
      .or_default()
      .insert(user_id.clone());

    debug!(
      user_id = %user_id,
      peer_id = %peer_id,
      "Active peer relationship established"
    );
  }

  /// Remove an active peer relationship.
  /// This should be called when a PeerClosed message is sent/received.
  pub fn remove_active_peer(&self, user_id: &UserId, peer_id: &UserId) {
    // Remove peer from user's peer set
    if let Some(mut peers) = self.active_peers.get_mut(user_id) {
      peers.remove(peer_id);
      if peers.is_empty() {
        drop(peers);
        self.active_peers.remove(user_id);
      }
    }

    // Also remove the reverse relationship
    if let Some(mut peers) = self.active_peers.get_mut(peer_id) {
      peers.remove(user_id);
      if peers.is_empty() {
        drop(peers);
        self.active_peers.remove(peer_id);
      }
    }

    debug!(
      user_id = %user_id,
      peer_id = %peer_id,
      "Active peer relationship removed"
    );
  }

  /// Get all active peers for a user.
  #[must_use]
  pub fn get_active_peers(&self, user_id: &UserId) -> Vec<UserId> {
    self
      .active_peers
      .get(user_id)
      .map(|entry| entry.value().iter().cloned().collect())
      .unwrap_or_default()
  }

  /// Check if two users have an active peer relationship.
  #[must_use]
  pub fn are_peers(&self, user_id: &UserId, peer_id: &UserId) -> bool {
    self
      .active_peers
      .get(user_id)
      .map(|entry| entry.value().contains(peer_id))
      .unwrap_or(false)
  }

  /// Remove all active peers for a user (e.g., on disconnect).
  pub fn clear_active_peers(&self, user_id: &UserId) {
    // Get all peers to notify
    let peers = self.get_active_peers(user_id);

    // Remove user from each peer's set
    for peer_id in &peers {
      if let Some(mut peer_set) = self.active_peers.get_mut(peer_id) {
        peer_set.remove(user_id);
        if peer_set.is_empty() {
          drop(peer_set);
          self.active_peers.remove(peer_id);
        }
      }
    }

    // Remove user's entry
    self.active_peers.remove(user_id);

    if !peers.is_empty() {
      debug!(
        user_id = %user_id,
        peer_count = peers.len(),
        "Cleared all active peer relationships"
      );
    }
  }

  // ===========================================================================
  // SDP Negotiation Management
  // ===========================================================================

  /// Start an SDP negotiation.
  /// Returns true if negotiation was started, false if one is already in progress.
  pub fn start_sdp_negotiation(&self, from: &UserId, to: &UserId) -> bool {
    let key = (from.clone(), to.clone());

    // Check if there's already an active negotiation
    if let Some(existing) = self.sdp_negotiations.get(&key)
      && existing.is_in_progress()
      && !existing.is_timed_out()
    {
      debug!(
        from = %from,
        to = %to,
        "SDP negotiation already in progress, queuing new offer"
      );
      return false;
    }

    // Create new negotiation state
    let state = SdpNegotiationState::new(from.clone(), to.clone());
    self.sdp_negotiations.insert(key, state);

    debug!(
      from = %from,
      to = %to,
      "SDP negotiation started"
    );

    true
  }

  /// Mark an SDP offer as sent.
  pub fn mark_offer_sent(&self, from: &UserId, to: &UserId) {
    let key = (from.clone(), to.clone());
    if let Some(mut state) = self.sdp_negotiations.get_mut(&key) {
      state.offer_sent = true;
      debug!(
        from = %from,
        to = %to,
        "SDP offer marked as sent"
      );
    }
  }

  /// Mark an SDP answer as received.
  pub fn mark_answer_received(&self, from: &UserId, to: &UserId) {
    let key = (from.clone(), to.clone());
    if let Some(mut state) = self.sdp_negotiations.get_mut(&key) {
      state.answer_received = true;
      debug!(
        from = %from,
        to = %to,
        "SDP answer marked as received"
      );
    }
  }

  /// Complete an SDP negotiation.
  pub fn complete_sdp_negotiation(&self, from: &UserId, to: &UserId) {
    let key = (from.clone(), to.clone());
    self.sdp_negotiations.remove(&key);
    debug!(
      from = %from,
      to = %to,
      "SDP negotiation completed"
    );
  }

  /// Check if there's an active SDP negotiation between two users.
  #[must_use]
  pub fn is_sdp_negotiation_in_progress(&self, from: &UserId, to: &UserId) -> bool {
    let key = (from.clone(), to.clone());
    self
      .sdp_negotiations
      .get(&key)
      .map(|state| state.is_in_progress() && !state.is_timed_out())
      .unwrap_or(false)
  }

  /// Get all pending SDP negotiations for a user (both as initiator and target).
  #[must_use]
  pub fn get_pending_sdp_negotiations(&self, user_id: &UserId) -> Vec<(UserId, UserId)> {
    self
      .sdp_negotiations
      .iter()
      .filter(|entry| {
        let (from, to) = entry.key();
        (from == user_id || to == user_id) && entry.value().is_in_progress()
      })
      .map(|entry| entry.key().clone())
      .collect()
  }

  /// Clean up expired SDP negotiations.
  pub fn cleanup_expired_sdp_negotiations(&self) {
    let keys_to_remove: Vec<(UserId, UserId)> = self
      .sdp_negotiations
      .iter()
      .filter(|entry| entry.value().is_timed_out())
      .map(|entry| entry.key().clone())
      .collect();

    for key in keys_to_remove {
      self.sdp_negotiations.remove(&key);
      debug!(
        from = %key.0,
        to = %key.1,
        "Removed expired SDP negotiation"
      );
    }
  }

  /// Clear all SDP negotiations for a user (e.g., on disconnect).
  pub fn clear_sdp_negotiations_for_user(&self, user_id: &UserId) {
    let keys_to_remove: Vec<(UserId, UserId)> = self
      .sdp_negotiations
      .iter()
      .filter(|entry| {
        let (from, to) = entry.key();
        from == user_id || to == user_id
      })
      .map(|entry| entry.key().clone())
      .collect();

    for key in keys_to_remove {
      self.sdp_negotiations.remove(&key);
      debug!(
        from = %key.0,
        to = %key.1,
        "Removed SDP negotiation for disconnected user"
      );
    }
  }
}

impl Default for DiscoveryState {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests;
