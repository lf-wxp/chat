/**
 * Room (multi-user chat room) E2E tests.
 *
 * Wave P0-6 of the E2E coverage plan. Maps to Requirement 04 — multi-user
 * Chat rooms (Theater rooms have their own future spec). Adds four
 * regression guards around the membership + conversation-listing
 * surface, which is the slice of the Room feature that is currently
 * complete end-to-end:
 *
 *   1. A creates a Chat room → it appears in A's sidebar room list, A is
 *      auto-joined (the join button is disabled / `data-joined="true"`),
 *      and A is auto-switched into the room conversation.
 *   2. B (a second user logged into the same server) sees the room in
 *      their own sidebar via `RoomListUpdate`. Clicking join flips the
 *      join button to disabled on B's side, mirroring the
 *      `joined_rooms` set derived from `app_state.room_members`.
 *   3. After both users are in the room, the room appears as a regular
 *      `sidebar-conversation-item` entry on BOTH users' sidebars with
 *      `data-conversation-type="room"` and the correct `data-room-id`.
 *   4. Clicking the conversation entry switches the chat view into the
 *      room on each user's side (verified via `active_conversation`
 *      reflecting in the item's `aria-pressed="true"`).
 *
 * --- Scope note ---
 * "Actually exchange a message over a room DataChannel" is deliberately
 * NOT covered here. The current wire protocol does not tag chat frames
 * with the originating `RoomId` (only file-transfer frames do — see
 * `message::datachannel::FileMetadata.room_id`). As a consequence, a
 * chat text sent in a room conversation is delivered but attributed on
 * the receiver side to the direct conversation with the sender, not to
 * the room. That is a feature gap in `message::datachannel` /
 * `frontend::webrtc::raw_frame`, not a flakiness issue with the test —
 * we intentionally avoid papering over it with an E2E test that would
 * require a bug fix to pass. A follow-up task should widen `ChatText`
 * (and its siblings) with an `Option<RoomId>` and update
 * `dispatch_incoming` to honour it; the corresponding "room message
 * round-trip" spec will land alongside that fix.
 *
 * Member-list add/remove UI assertions are similarly deferred to a
 * future Theater-focused spec — the regular Chat room ChatView does
 * not mount a `MemberListPanel` (only Theater rooms do). The
 * membership-change signal is captured here via the `data-joined`
 * attribute on `sidebar-room-item`, which is bound to the same
 * `room_members` map.
 */

import { sel } from '../utils/selectors.ts';
import { createRoom, joinRoomByName, registerAndLogin } from '../fixtures/helpers.ts';
import { expect, test } from '../fixtures/test-base.ts';

test.describe('rooms (chat type)', () => {
  test('A creates a Chat room — appears in sidebar, A auto-joined', async ({ pageA, server }) => {
    await registerAndLogin(pageA, server, { hint: 'room-c-a' });
    const room = await createRoom(pageA, { description: 'P0-6 test room' });

    // Sidebar room item exists with data-joined="true".
    const item = pageA.locator(room.itemSelector);
    await expect(item).toBeVisible();
    await expect(item).toHaveAttribute('data-joined', 'true', { timeout: 10_000 });

    // The join button is disabled (the user is already a member).
    const joinBtn = item.locator(sel.sidebarRoomJoinBtn);
    await expect(joinBtn).toBeDisabled();

    // The chat view is open for the new room (auto-switched by the
    // `RoomCreated` handler).
    await expect(pageA.locator(sel.chatView)).toBeVisible();
  });

  test('B sees the room and joining flips the data-joined attribute', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'room-j-a' });
    await registerAndLogin(pageB, server, { hint: 'room-j-b' });
    const room = await createRoom(pageA, { description: 'join target' });

    // B sees the room appear in their sidebar (RoomListUpdate path).
    const itemOnB = pageB.locator(room.itemSelector);
    await expect(itemOnB).toBeVisible({ timeout: 15_000 });
    // B is NOT a member yet — data-joined="false".
    await expect(itemOnB).toHaveAttribute('data-joined', 'false');

    await joinRoomByName(pageB, room.name);

    // Post-join: data-joined flipped to true on B's side.
    await expect(itemOnB).toHaveAttribute('data-joined', 'true', { timeout: 15_000 });
    await expect(itemOnB.locator(sel.sidebarRoomJoinBtn)).toBeDisabled();
  });

  test('after both join, the room is listed as a sidebar conversation on each side', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'room-list-a' });
    await registerAndLogin(pageB, server, { hint: 'room-list-b' });
    const room = await createRoom(pageA, {});
    const roomId = await pageA.locator(room.itemSelector).getAttribute('data-room-id');
    expect(roomId).not.toBeNull();

    await joinRoomByName(pageB, room.name);

    // On A's side: the room conversation was materialised by
    // `RoomCreated` → `ensure_room_conversation`.
    const convOnA = pageA.locator(
      `${sel.sidebarConversationItem}[data-room-id="${roomId}"][data-conversation-type="room"]`,
    );
    await expect(convOnA).toBeVisible({ timeout: 15_000 });

    // On B's side: same materialisation runs via `RoomJoined`.
    const convOnB = pageB.locator(
      `${sel.sidebarConversationItem}[data-room-id="${roomId}"][data-conversation-type="room"]`,
    );
    await expect(convOnB).toBeVisible({ timeout: 15_000 });
  });

  test('clicking the room conversation entry activates the chat view on each side', async ({
    pageA,
    pageB,
    server,
  }) => {
    await registerAndLogin(pageA, server, { hint: 'room-active-a' });
    await registerAndLogin(pageB, server, { hint: 'room-active-b' });
    const room = await createRoom(pageA, {});
    const roomId = await pageA.locator(room.itemSelector).getAttribute('data-room-id');
    expect(roomId).not.toBeNull();

    await joinRoomByName(pageB, room.name);

    const roomConvSelector = `${sel.sidebarConversationItem}[data-room-id="${roomId}"]`;
    await pageA.locator(roomConvSelector).click();
    await pageB.locator(roomConvSelector).click();

    // `aria-pressed="true"` is bound to `active_conversation` in
    // `SidebarConversationItem` — our contractual signal that the
    // conversation is the one currently rendered by `<HomePage>`.
    await expect(pageA.locator(roomConvSelector)).toHaveAttribute(
      'aria-pressed',
      'true',
      { timeout: 10_000 },
    );
    await expect(pageB.locator(roomConvSelector)).toHaveAttribute(
      'aria-pressed',
      'true',
      { timeout: 10_000 },
    );

    // Both sides show a chat view (rather than the empty-home state
    // or a theater page).
    await expect(pageA.locator(sel.chatView)).toBeVisible();
    await expect(pageB.locator(sel.chatView)).toBeVisible();
  });
});
