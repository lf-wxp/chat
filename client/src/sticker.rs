//! Sticker pack registry — maps pack/sticker IDs to SVG asset paths.
//!
//! Each sticker is identified by `(pack_id, sticker_id)` and resolved to a
//! static SVG URL served from `/stickers/<pack_id>/<sticker_id>.svg`.

/// A single sticker entry in a pack.
#[derive(Debug, Clone)]
pub struct StickerEntry {
  /// Unique sticker identifier within the pack (also the SVG filename without extension).
  pub id: &'static str,
  /// Human-readable label (used as `alt` / `title` text).
  pub label: &'static str,
}

/// A sticker pack containing multiple stickers.
#[derive(Debug, Clone)]
pub struct StickerPack {
  /// Unique pack identifier (also the directory name under `/stickers/`).
  pub id: &'static str,
  /// Display name shown in the sticker panel tab.
  pub name: &'static str,
  /// Ordered list of stickers in this pack.
  pub stickers: &'static [StickerEntry],
}

/// Built-in default sticker pack — 20 expressive SVG stickers.
pub const DEFAULT_PACK: StickerPack = StickerPack {
  id: "default",
  name: "Default",
  stickers: &[
    StickerEntry {
      id: "wave",
      label: "Wave",
    },
    StickerEntry {
      id: "party",
      label: "Party",
    },
    StickerEntry {
      id: "heart",
      label: "Heart",
    },
    StickerEntry {
      id: "sad",
      label: "Sad",
    },
    StickerEntry {
      id: "laugh",
      label: "Laugh",
    },
    StickerEntry {
      id: "think",
      label: "Think",
    },
    StickerEntry {
      id: "cool",
      label: "Cool",
    },
    StickerEntry {
      id: "angry",
      label: "Angry",
    },
    StickerEntry {
      id: "ghost",
      label: "Ghost",
    },
    StickerEntry {
      id: "fire",
      label: "Fire",
    },
    StickerEntry {
      id: "star",
      label: "Star",
    },
    StickerEntry {
      id: "trophy",
      label: "Trophy",
    },
    StickerEntry {
      id: "target",
      label: "Target",
    },
    StickerEntry {
      id: "bear",
      label: "Bear",
    },
    StickerEntry {
      id: "clover",
      label: "Clover",
    },
    StickerEntry {
      id: "sunflower",
      label: "Sunflower",
    },
    StickerEntry {
      id: "butterfly",
      label: "Butterfly",
    },
    StickerEntry {
      id: "paw",
      label: "Paw",
    },
    StickerEntry {
      id: "rocket",
      label: "Rocket",
    },
    StickerEntry {
      id: "unicorn",
      label: "Unicorn",
    },
    StickerEntry {
      id: "dragon",
      label: "Dragon",
    },
    StickerEntry {
      id: "sparkle",
      label: "Sparkle",
    },
    StickerEntry {
      id: "pumpkin",
      label: "Pumpkin",
    },
    StickerEntry {
      id: "rainbow",
      label: "Rainbow",
    },
  ],
};

/// All available sticker packs.
pub const ALL_PACKS: &[StickerPack] = &[DEFAULT_PACK];

/// Resolve a `(pack_id, sticker_id)` pair to the SVG asset URL.
///
/// Returns `None` if the pack or sticker is not found in the registry.
pub fn sticker_url(pack_id: &str, sticker_id: &str) -> Option<String> {
  for pack in ALL_PACKS {
    if pack.id == pack_id {
      for sticker in pack.stickers {
        if sticker.id == sticker_id {
          return Some(format!("/stickers/{pack_id}/{sticker_id}.svg"));
        }
      }
    }
  }
  None
}

/// Get the label for a sticker by its pack and sticker ID.
pub fn sticker_label(pack_id: &str, sticker_id: &str) -> Option<&'static str> {
  for pack in ALL_PACKS {
    if pack.id == pack_id {
      for sticker in pack.stickers {
        if sticker.id == sticker_id {
          return Some(sticker.label);
        }
      }
    }
  }
  None
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
  use super::*;

  // =========================================================================
  // DEFAULT_PACK integrity tests
  // =========================================================================

  #[test]
  fn test_default_pack_has_stickers() {
    assert!(!DEFAULT_PACK.stickers.is_empty());
    assert_eq!(DEFAULT_PACK.id, "default");
    assert_eq!(DEFAULT_PACK.name, "Default");
  }

  #[test]
  fn test_default_pack_sticker_count() {
    assert_eq!(DEFAULT_PACK.stickers.len(), 24);
  }

  #[test]
  fn test_default_pack_unique_ids() {
    let ids: Vec<&str> = DEFAULT_PACK.stickers.iter().map(|s| s.id).collect();
    let mut deduped = ids.clone();
    deduped.sort();
    deduped.dedup();
    assert_eq!(ids.len(), deduped.len(), "Sticker IDs must be unique");
  }

  #[test]
  fn test_all_packs_contains_default() {
    assert_eq!(ALL_PACKS.len(), 1);
    assert_eq!(ALL_PACKS[0].id, "default");
  }

  // =========================================================================
  // sticker_url tests
  // =========================================================================

  #[test]
  fn test_sticker_url_valid() {
    let url = sticker_url("default", "wave");
    assert_eq!(url, Some("/stickers/default/wave.svg".to_string()));
  }

  #[test]
  fn test_sticker_url_all_default_stickers() {
    for sticker in DEFAULT_PACK.stickers {
      let url = sticker_url("default", sticker.id);
      assert!(
        url.is_some(),
        "sticker_url should resolve for {}",
        sticker.id
      );
      assert_eq!(
        url.unwrap(),
        format!("/stickers/default/{}.svg", sticker.id)
      );
    }
  }

  #[test]
  fn test_sticker_url_unknown_pack() {
    assert_eq!(sticker_url("nonexistent", "wave"), None);
  }

  #[test]
  fn test_sticker_url_unknown_sticker() {
    assert_eq!(sticker_url("default", "nonexistent"), None);
  }

  // =========================================================================
  // sticker_label tests
  // =========================================================================

  #[test]
  fn test_sticker_label_valid() {
    assert_eq!(sticker_label("default", "wave"), Some("Wave"));
    assert_eq!(sticker_label("default", "party"), Some("Party"));
    assert_eq!(sticker_label("default", "heart"), Some("Heart"));
    assert_eq!(sticker_label("default", "dragon"), Some("Dragon"));
  }

  #[test]
  fn test_sticker_label_unknown_pack() {
    assert_eq!(sticker_label("nonexistent", "wave"), None);
  }

  #[test]
  fn test_sticker_label_unknown_sticker() {
    assert_eq!(sticker_label("default", "nonexistent"), None);
  }
}
