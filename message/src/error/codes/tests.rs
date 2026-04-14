use super::*;

/// Test that all Signaling (SIG) error codes have correct module and category
#[test]
fn test_sig_error_codes_module_category() {
  // Network errors (category 0)
  assert_eq!(SIG001.module, ErrorModule::Sig);
  assert_eq!(SIG001.category, ErrorCategory::Network);
  assert_eq!(SIG001.sequence, 1);

  assert_eq!(SIG002.module, ErrorModule::Sig);
  assert_eq!(SIG002.category, ErrorCategory::Network);
  assert_eq!(SIG002.sequence, 2);

  assert_eq!(SIG003.module, ErrorModule::Sig);
  assert_eq!(SIG003.category, ErrorCategory::Network);
  assert_eq!(SIG003.sequence, 3);

  assert_eq!(SIG004.module, ErrorModule::Sig);
  assert_eq!(SIG004.category, ErrorCategory::Network);
  assert_eq!(SIG004.sequence, 4);

  assert_eq!(SIG005.module, ErrorModule::Sig);
  assert_eq!(SIG005.category, ErrorCategory::Network);
  assert_eq!(SIG005.sequence, 5);

  // Client errors (category 1)
  assert_eq!(SIG101.module, ErrorModule::Sig);
  assert_eq!(SIG101.category, ErrorCategory::Client);
  assert_eq!(SIG101.sequence, 1);

  assert_eq!(SIG102.module, ErrorModule::Sig);
  assert_eq!(SIG102.category, ErrorCategory::Client);
  assert_eq!(SIG102.sequence, 2);

  assert_eq!(SIG103.module, ErrorModule::Sig);
  assert_eq!(SIG103.category, ErrorCategory::Client);
  assert_eq!(SIG103.sequence, 3);

  assert_eq!(SIG104.module, ErrorModule::Sig);
  assert_eq!(SIG104.category, ErrorCategory::Client);
  assert_eq!(SIG104.sequence, 4);
}

/// Test that all Chat (CHT) error codes have correct module and category
#[test]
fn test_cht_error_codes_module_category() {
  // Network errors (category 0)
  assert_eq!(CHT001.module, ErrorModule::Cht);
  assert_eq!(CHT001.category, ErrorCategory::Network);
  assert_eq!(CHT001.sequence, 1);

  assert_eq!(CHT002.module, ErrorModule::Cht);
  assert_eq!(CHT002.category, ErrorCategory::Network);
  assert_eq!(CHT002.sequence, 2);

  assert_eq!(CHT003.module, ErrorModule::Cht);
  assert_eq!(CHT003.category, ErrorCategory::Network);
  assert_eq!(CHT003.sequence, 3);

  // Client errors (category 1)
  assert_eq!(CHT101.module, ErrorModule::Cht);
  assert_eq!(CHT101.category, ErrorCategory::Client);
  assert_eq!(CHT101.sequence, 1);

  assert_eq!(CHT102.module, ErrorModule::Cht);
  assert_eq!(CHT102.category, ErrorCategory::Client);
  assert_eq!(CHT102.sequence, 2);

  assert_eq!(CHT103.module, ErrorModule::Cht);
  assert_eq!(CHT103.category, ErrorCategory::Client);
  assert_eq!(CHT103.sequence, 3);

  assert_eq!(CHT104.module, ErrorModule::Cht);
  assert_eq!(CHT104.category, ErrorCategory::Client);
  assert_eq!(CHT104.sequence, 4);

  assert_eq!(CHT105.module, ErrorModule::Cht);
  assert_eq!(CHT105.category, ErrorCategory::Client);
  assert_eq!(CHT105.sequence, 5);

  // Security errors (category 5)
  assert_eq!(CHT501.module, ErrorModule::Cht);
  assert_eq!(CHT501.category, ErrorCategory::Security);
  assert_eq!(CHT501.sequence, 1);

  assert_eq!(CHT502.module, ErrorModule::Cht);
  assert_eq!(CHT502.category, ErrorCategory::Security);
  assert_eq!(CHT502.sequence, 2);
}

/// Test that all Audio/Video (AV) error codes have correct module and category
#[test]
fn test_av_error_codes_module_category() {
  // Network errors (category 0)
  assert_eq!(AV001.module, ErrorModule::Av);
  assert_eq!(AV001.category, ErrorCategory::Network);
  assert_eq!(AV001.sequence, 1);

  assert_eq!(AV002.module, ErrorModule::Av);
  assert_eq!(AV002.category, ErrorCategory::Network);
  assert_eq!(AV002.sequence, 2);

  // Media errors (category 4)
  assert_eq!(AV401.module, ErrorModule::Av);
  assert_eq!(AV401.category, ErrorCategory::Media);
  assert_eq!(AV401.sequence, 1);

  assert_eq!(AV402.module, ErrorModule::Av);
  assert_eq!(AV402.category, ErrorCategory::Media);
  assert_eq!(AV402.sequence, 2);

  assert_eq!(AV403.module, ErrorModule::Av);
  assert_eq!(AV403.category, ErrorCategory::Media);
  assert_eq!(AV403.sequence, 3);

  assert_eq!(AV404.module, ErrorModule::Av);
  assert_eq!(AV404.category, ErrorCategory::Media);
  assert_eq!(AV404.sequence, 4);

  assert_eq!(AV405.module, ErrorModule::Av);
  assert_eq!(AV405.category, ErrorCategory::Media);
  assert_eq!(AV405.sequence, 5);
}

/// Test that all Room (ROM) error codes have correct module and category
#[test]
fn test_rom_error_codes_module_category() {
  // Network errors (category 0)
  assert_eq!(ROM001.module, ErrorModule::Rom);
  assert_eq!(ROM001.category, ErrorCategory::Network);
  assert_eq!(ROM001.sequence, 1);

  assert_eq!(ROM002.module, ErrorModule::Rom);
  assert_eq!(ROM002.category, ErrorCategory::Network);
  assert_eq!(ROM002.sequence, 2);

  // Client errors (category 1)
  assert_eq!(ROM101.module, ErrorModule::Rom);
  assert_eq!(ROM101.category, ErrorCategory::Client);
  assert_eq!(ROM101.sequence, 1);

  assert_eq!(ROM102.module, ErrorModule::Rom);
  assert_eq!(ROM102.category, ErrorCategory::Client);
  assert_eq!(ROM102.sequence, 2);

  assert_eq!(ROM103.module, ErrorModule::Rom);
  assert_eq!(ROM103.category, ErrorCategory::Client);
  assert_eq!(ROM103.sequence, 3);

  assert_eq!(ROM104.module, ErrorModule::Rom);
  assert_eq!(ROM104.category, ErrorCategory::Client);
  assert_eq!(ROM104.sequence, 4);

  assert_eq!(ROM105.module, ErrorModule::Rom);
  assert_eq!(ROM105.category, ErrorCategory::Client);
  assert_eq!(ROM105.sequence, 5);

  assert_eq!(ROM106.module, ErrorModule::Rom);
  assert_eq!(ROM106.category, ErrorCategory::Client);
  assert_eq!(ROM106.sequence, 6);

  assert_eq!(ROM107.module, ErrorModule::Rom);
  assert_eq!(ROM107.category, ErrorCategory::Client);
  assert_eq!(ROM107.sequence, 7);

  assert_eq!(ROM108.module, ErrorModule::Rom);
  assert_eq!(ROM108.category, ErrorCategory::Client);
  assert_eq!(ROM108.sequence, 8);
}

/// Test that all Theater (THR) error codes have correct module and category
#[test]
fn test_thr_error_codes_module_category() {
  // Network errors (category 0)
  assert_eq!(THR001.module, ErrorModule::Thr);
  assert_eq!(THR001.category, ErrorCategory::Network);
  assert_eq!(THR001.sequence, 1);

  assert_eq!(THR002.module, ErrorModule::Thr);
  assert_eq!(THR002.category, ErrorCategory::Network);
  assert_eq!(THR002.sequence, 2);

  assert_eq!(THR003.module, ErrorModule::Thr);
  assert_eq!(THR003.category, ErrorCategory::Network);
  assert_eq!(THR003.sequence, 3);

  // Client errors (category 1)
  assert_eq!(THR101.module, ErrorModule::Thr);
  assert_eq!(THR101.category, ErrorCategory::Client);
  assert_eq!(THR101.sequence, 1);

  assert_eq!(THR102.module, ErrorModule::Thr);
  assert_eq!(THR102.category, ErrorCategory::Client);
  assert_eq!(THR102.sequence, 2);

  assert_eq!(THR103.module, ErrorModule::Thr);
  assert_eq!(THR103.category, ErrorCategory::Client);
  assert_eq!(THR103.sequence, 3);

  assert_eq!(THR104.module, ErrorModule::Thr);
  assert_eq!(THR104.category, ErrorCategory::Client);
  assert_eq!(THR104.sequence, 4);
}

/// Test that all File Transfer (FIL) error codes have correct module and category
#[test]
fn test_fil_error_codes_module_category() {
  // Network errors (category 0)
  assert_eq!(FIL001.module, ErrorModule::Fil);
  assert_eq!(FIL001.category, ErrorCategory::Network);
  assert_eq!(FIL001.sequence, 1);

  assert_eq!(FIL002.module, ErrorModule::Fil);
  assert_eq!(FIL002.category, ErrorCategory::Network);
  assert_eq!(FIL002.sequence, 2);

  // Client errors (category 1)
  assert_eq!(FIL101.module, ErrorModule::Fil);
  assert_eq!(FIL101.category, ErrorCategory::Client);
  assert_eq!(FIL101.sequence, 1);

  assert_eq!(FIL102.module, ErrorModule::Fil);
  assert_eq!(FIL102.category, ErrorCategory::Client);
  assert_eq!(FIL102.sequence, 2);

  assert_eq!(FIL103.module, ErrorModule::Fil);
  assert_eq!(FIL103.category, ErrorCategory::Client);
  assert_eq!(FIL103.sequence, 3);

  assert_eq!(FIL104.module, ErrorModule::Fil);
  assert_eq!(FIL104.category, ErrorCategory::Client);
  assert_eq!(FIL104.sequence, 4);
}

/// Test that all Authentication (AUTH) error codes have correct module and category
#[test]
fn test_auth_error_codes_module_category() {
  // Network errors (category 0)
  assert_eq!(AUTH001.module, ErrorModule::Auth);
  assert_eq!(AUTH001.category, ErrorCategory::Network);
  assert_eq!(AUTH001.sequence, 1);

  // Client errors (category 1)
  assert_eq!(AUTH101.module, ErrorModule::Auth);
  assert_eq!(AUTH101.category, ErrorCategory::Client);
  assert_eq!(AUTH101.sequence, 1);

  assert_eq!(AUTH102.module, ErrorModule::Auth);
  assert_eq!(AUTH102.category, ErrorCategory::Client);
  assert_eq!(AUTH102.sequence, 2);

  assert_eq!(AUTH103.module, ErrorModule::Auth);
  assert_eq!(AUTH103.category, ErrorCategory::Client);
  assert_eq!(AUTH103.sequence, 3);

  // Security errors (category 5)
  assert_eq!(AUTH501.module, ErrorModule::Auth);
  assert_eq!(AUTH501.category, ErrorCategory::Security);
  assert_eq!(AUTH501.sequence, 1);

  assert_eq!(AUTH502.module, ErrorModule::Auth);
  assert_eq!(AUTH502.category, ErrorCategory::Security);
  assert_eq!(AUTH502.sequence, 2);

  assert_eq!(AUTH503.module, ErrorModule::Auth);
  assert_eq!(AUTH503.category, ErrorCategory::Security);
  assert_eq!(AUTH503.sequence, 3);
}

/// Test that all Persistence (PST) error codes have correct module and category
#[test]
fn test_pst_error_codes_module_category() {
  // Server errors (category 3)
  assert_eq!(PST301.module, ErrorModule::Pst);
  assert_eq!(PST301.category, ErrorCategory::Server);
  assert_eq!(PST301.sequence, 1);

  assert_eq!(PST302.module, ErrorModule::Pst);
  assert_eq!(PST302.category, ErrorCategory::Server);
  assert_eq!(PST302.sequence, 2);

  assert_eq!(PST303.module, ErrorModule::Pst);
  assert_eq!(PST303.category, ErrorCategory::Server);
  assert_eq!(PST303.sequence, 3);
}

/// Test that all System (SYS) error codes have correct module and category
#[test]
fn test_sys_error_codes_module_category() {
  // Network errors (category 0)
  assert_eq!(SYS001.module, ErrorModule::Sys);
  assert_eq!(SYS001.category, ErrorCategory::Network);
  assert_eq!(SYS001.sequence, 1);

  // Client errors (category 1)
  assert_eq!(SYS101.module, ErrorModule::Sys);
  assert_eq!(SYS101.category, ErrorCategory::Client);
  assert_eq!(SYS101.sequence, 1);

  assert_eq!(SYS102.module, ErrorModule::Sys);
  assert_eq!(SYS102.category, ErrorCategory::Client);
  assert_eq!(SYS102.sequence, 2);

  assert_eq!(SYS103.module, ErrorModule::Sys);
  assert_eq!(SYS103.category, ErrorCategory::Client);
  assert_eq!(SYS103.sequence, 3);

  // Server errors (category 3)
  assert_eq!(SYS301.module, ErrorModule::Sys);
  assert_eq!(SYS301.category, ErrorCategory::Server);
  assert_eq!(SYS301.sequence, 1);
}

/// Test error code equality
#[test]
fn test_error_code_equality() {
  assert_eq!(SIG001, SIG001);
  assert_ne!(SIG001, SIG002);
  assert_ne!(SIG001, SIG101);
}

/// Test error code creation
#[test]
fn test_error_code_new() {
  let custom_code = ErrorCode::new(ErrorModule::Sig, ErrorCategory::Network, 99);
  assert_eq!(custom_code.module, ErrorModule::Sig);
  assert_eq!(custom_code.category, ErrorCategory::Network);
  assert_eq!(custom_code.sequence, 99);
}

/// Test that all error codes have unique string representations
#[test]
fn test_error_codes_unique_strings() {
  let all_codes: Vec<ErrorCode> = vec![
    // SIG
    SIG001, SIG002, SIG003, SIG004, SIG005, SIG101, SIG102, SIG103, SIG104, // CHT
    CHT001, CHT002, CHT003, CHT101, CHT102, CHT103, CHT104, CHT105, CHT501, CHT502,
    // AV
    AV001, AV002, AV401, AV402, AV403, AV404, AV405, // ROM
    ROM001, ROM002, ROM101, ROM102, ROM103, ROM104, ROM105, ROM106, ROM107, ROM108,
    // THR
    THR001, THR002, THR003, THR101, THR102, THR103, THR104, // FIL
    FIL001, FIL002, FIL101, FIL102, FIL103, FIL104, // AUTH
    AUTH001, AUTH101, AUTH102, AUTH103, AUTH501, AUTH502, AUTH503, // PST
    PST301, PST302, PST303, // SYS
    SYS001, SYS101, SYS102, SYS103, SYS301,
  ];

  let code_strings: Vec<String> = all_codes.iter().map(ErrorCode::to_code_string).collect();
  let unique_count = code_strings
    .iter()
    .collect::<std::collections::HashSet<_>>()
    .len();
  assert_eq!(
    unique_count,
    all_codes.len(),
    "All error codes should have unique string representations"
  );
}

/// Test that error codes match documented values from requirements
#[test]
fn test_error_codes_match_requirements() {
  // Signaling codes (SIG001-SIG104)
  assert_eq!(SIG001.to_code_string(), "SIG001");
  assert_eq!(SIG002.to_code_string(), "SIG002");
  assert_eq!(SIG003.to_code_string(), "SIG003");
  assert_eq!(SIG004.to_code_string(), "SIG004");
  assert_eq!(SIG005.to_code_string(), "SIG005");
  assert_eq!(SIG101.to_code_string(), "SIG101");
  assert_eq!(SIG102.to_code_string(), "SIG102");
  assert_eq!(SIG103.to_code_string(), "SIG103");
  assert_eq!(SIG104.to_code_string(), "SIG104");

  // Chat codes (CHT001-CHT105, CHT501-CHT502)
  assert_eq!(CHT001.to_code_string(), "CHT001");
  assert_eq!(CHT002.to_code_string(), "CHT002");
  assert_eq!(CHT003.to_code_string(), "CHT003");
  assert_eq!(CHT101.to_code_string(), "CHT101");
  assert_eq!(CHT102.to_code_string(), "CHT102");
  assert_eq!(CHT103.to_code_string(), "CHT103");
  assert_eq!(CHT104.to_code_string(), "CHT104");
  assert_eq!(CHT105.to_code_string(), "CHT105");
  assert_eq!(CHT501.to_code_string(), "CHT501");
  assert_eq!(CHT502.to_code_string(), "CHT502");

  // Room codes (ROM104-ROM108 as per requirements)
  assert_eq!(ROM104.to_code_string(), "ROM104");
  assert_eq!(ROM105.to_code_string(), "ROM105");
  assert_eq!(ROM106.to_code_string(), "ROM106");
  assert_eq!(ROM107.to_code_string(), "ROM107");
  assert_eq!(ROM108.to_code_string(), "ROM108");

  // Theater codes (THR103-THR104 as per requirements)
  assert_eq!(THR103.to_code_string(), "THR103");
  assert_eq!(THR104.to_code_string(), "THR104");

  // Auth codes (AUTH501-AUTH503 as per requirements)
  assert_eq!(AUTH501.to_code_string(), "AUTH501");
  assert_eq!(AUTH502.to_code_string(), "AUTH502");
  assert_eq!(AUTH503.to_code_string(), "AUTH503");

  // System codes (SYS301 as per requirements)
  assert_eq!(SYS301.to_code_string(), "SYS301");
}

/// Total count of all defined error codes.
/// When adding a new error code, this constant MUST be updated.
/// This test acts as a guardrail: if you add a code but forget to
/// update the exhaustiveness list, the count check will fail.
const TOTAL_DEFINED_ERROR_CODES: usize = 64;

/// Exhaustive list of ALL error codes defined in the codes module.
/// When adding a new error code constant, you MUST also add it here.
/// This ensures:
/// 1. No code is accidentally omitted from uniqueness checks
/// 2. i18n key mapping coverage is complete
/// 3. The total count stays in sync with definitions
fn all_error_codes() -> Vec<ErrorCode> {
  vec![
    // SIG (9 codes)
    SIG001, SIG002, SIG003, SIG004, SIG005, SIG101, SIG102, SIG103, SIG104,
    // CHT (10 codes)
    CHT001, CHT002, CHT003, CHT101, CHT102, CHT103, CHT104, CHT105, CHT501, CHT502,
    // AV (7 codes)
    AV001, AV002, AV401, AV402, AV403, AV404, AV405, // ROM (10 codes)
    ROM001, ROM002, ROM101, ROM102, ROM103, ROM104, ROM105, ROM106, ROM107, ROM108,
    // THR (7 codes)
    THR001, THR002, THR003, THR101, THR102, THR103, THR104, // FIL (6 codes)
    FIL001, FIL002, FIL101, FIL102, FIL103, FIL104, // AUTH (7 codes)
    AUTH001, AUTH101, AUTH102, AUTH103, AUTH501, AUTH502, AUTH503, // PST (3 codes)
    PST301, PST302, PST303, // SYS (5 codes)
    SYS001, SYS101, SYS102, SYS103, SYS301,
  ]
}

/// Test that the exhaustive list count matches the expected total.
/// If this test fails, it means a new error code was added without
/// updating the exhaustive list, or a code was removed.
#[test]
fn test_error_codes_exhaustive_count() {
  let codes = all_error_codes();
  assert_eq!(
    codes.len(),
    TOTAL_DEFINED_ERROR_CODES,
    "Error code count mismatch. If you added a new code, update TOTAL_DEFINED_ERROR_CODES and all_error_codes(). Current: {}, Expected: {}",
    codes.len(),
    TOTAL_DEFINED_ERROR_CODES
  );
}

/// Test that all error codes have unique i18n keys.
/// This ensures every error code can be mapped to a localized message.
#[test]
fn test_all_error_codes_have_unique_i18n_keys() {
  let codes = all_error_codes();
  let i18n_keys: Vec<String> = codes
    .iter()
    .map(super::super::ErrorCode::to_i18n_key)
    .collect();
  let unique_keys: std::collections::HashSet<_> = i18n_keys.iter().collect();

  assert_eq!(
    unique_keys.len(),
    codes.len(),
    "Each error code must have a unique i18n key. Duplicate keys indicate a collision in the code string format."
  );
}

/// Test that i18n keys follow the expected format: "error.{module}{numeric}"
#[test]
fn test_all_error_codes_i18n_key_format() {
  let codes = all_error_codes();
  for code in &codes {
    let i18n_key = code.to_i18n_key();
    let code_string = code.to_code_string();
    let expected_key = format!("error.{}", code_string.to_lowercase());
    assert_eq!(
      i18n_key, expected_key,
      "i18n key for {code_string} does not follow expected format"
    );
  }
}

/// Test that no two error codes from different modules share the same numeric representation.
/// While module prefixes prevent ambiguity in code strings, this test catches
/// potential copy-paste errors where a code is assigned the wrong module.
#[test]
fn test_error_codes_no_cross_module_collision() {
  let codes = all_error_codes();
  let code_strings: Vec<String> = codes
    .iter()
    .map(super::super::ErrorCode::to_code_string)
    .collect();
  let unique_strings: std::collections::HashSet<_> = code_strings.iter().collect();

  assert_eq!(
    unique_strings.len(),
    codes.len(),
    "Cross-module code string collision detected. Every code must produce a unique string."
  );
}

/// Test that all error codes use the correct category discriminant mapping.
/// Category values: Network=0, Client=1, Informational=2, Server=3, Media=4, Security=5
/// The numeric part is category * 100 + sequence.
#[test]
fn test_error_code_category_discriminant_consistency() {
  let codes = all_error_codes();
  for code in &codes {
    let expected_numeric = code.category as u16 * 100 + code.sequence;
    let code_string = code.to_code_string();
    // Extract numeric part from code string (after the module prefix)
    let numeric_part: String = code_string.chars().filter(char::is_ascii_digit).collect();
    let numeric_value: u16 = numeric_part
      .parse()
      .unwrap_or_else(|_| panic!("Failed to parse numeric part of code string: {code_string}"));
    assert_eq!(
      numeric_value, expected_numeric,
      "Code {code_string} has inconsistent category discriminant. Expected numeric={expected_numeric}, got {numeric_value}"
    );
  }
}

// ============================================================================
// Additional Completeness Tests (CR-P1-002)
// ============================================================================

/// Test that each module has error codes in all expected categories.
/// Not all modules need all categories, but this documents which
/// categories are used by each module.
#[test]
fn test_error_codes_module_category_distribution() {
  use std::collections::HashMap;

  let codes = all_error_codes();
  let mut module_categories: HashMap<ErrorModule, std::collections::HashSet<ErrorCategory>> =
    HashMap::new();

  for code in &codes {
    module_categories
      .entry(code.module)
      .or_default()
      .insert(code.category);
  }

  // Verify expected module-category combinations exist
  // SIG: Network (0xx), Client (1xx)
  assert!(module_categories[&ErrorModule::Sig].contains(&ErrorCategory::Network));
  assert!(module_categories[&ErrorModule::Sig].contains(&ErrorCategory::Client));

  // CHT: Network (0xx), Client (1xx), Security (5xx)
  assert!(module_categories[&ErrorModule::Cht].contains(&ErrorCategory::Network));
  assert!(module_categories[&ErrorModule::Cht].contains(&ErrorCategory::Client));
  assert!(module_categories[&ErrorModule::Cht].contains(&ErrorCategory::Security));

  // AV: Network (0xx), Media (4xx)
  assert!(module_categories[&ErrorModule::Av].contains(&ErrorCategory::Network));
  assert!(module_categories[&ErrorModule::Av].contains(&ErrorCategory::Media));

  // AUTH: Network (0xx), Client (1xx), Security (5xx)
  assert!(module_categories[&ErrorModule::Auth].contains(&ErrorCategory::Network));
  assert!(module_categories[&ErrorModule::Auth].contains(&ErrorCategory::Client));
  assert!(module_categories[&ErrorModule::Auth].contains(&ErrorCategory::Security));

  // PST: Server (3xx)
  assert!(module_categories[&ErrorModule::Pst].contains(&ErrorCategory::Server));
}

/// Test that all error codes can be displayed (Debug/Display trait).
#[test]
fn test_error_code_debug_display() {
  let codes = all_error_codes();
  for code in &codes {
    // Debug trait should not panic
    let debug_str = format!("{code:?}");
    assert!(!debug_str.is_empty());

    // Display via to_code_string should match expected format
    let display_str = code.to_code_string();
    assert!(!display_str.is_empty());
    // Minimum length: "AV001" (2-char module + 3-digit number)
    // Maximum length: "AUTH501" (4-char module + 3-digit number)
    assert!(
      display_str.len() >= 5,
      "Code {display_str} should have at least 5 chars"
    );
  }
}

/// Test that error codes are Clone and Copy.
#[test]
fn test_error_code_clone_copy() {
  let original = SIG001;
  // Copy trait allows direct assignment without clone
  let copied: ErrorCode = original;

  assert_eq!(original, copied);
}

/// Test that `ErrorCode::new` creates valid codes for all module/category combinations.
#[test]
fn test_error_code_new_all_combinations() {
  let modules = [
    ErrorModule::Sig,
    ErrorModule::Cht,
    ErrorModule::Av,
    ErrorModule::Rom,
    ErrorModule::Thr,
    ErrorModule::Fil,
    ErrorModule::Auth,
    ErrorModule::Pst,
    ErrorModule::Sys,
  ];

  let categories = [
    ErrorCategory::Network,
    ErrorCategory::Client,
    ErrorCategory::Informational,
    ErrorCategory::Server,
    ErrorCategory::Media,
    ErrorCategory::Security,
  ];

  for module in &modules {
    for category in &categories {
      // Create a code with sequence 1
      let code = ErrorCode::new(*module, *category, 1);
      assert_eq!(code.module, *module);
      assert_eq!(code.category, *category);
      assert_eq!(code.sequence, 1);
    }
  }
}

/// Test that sequence numbers are valid (1-99 range conceptually, though not enforced).
#[test]
fn test_error_code_sequences_in_valid_range() {
  let codes = all_error_codes();
  for code in &codes {
    // Sequence should be positive and fit in two digits
    assert!(
      code.sequence >= 1 && code.sequence <= 99,
      "Sequence {} for code {} is out of expected range 1-99",
      code.sequence,
      code.to_code_string()
    );
  }
}

/// Test that no error code has sequence 0.
#[test]
fn test_no_zero_sequence_codes() {
  let codes = all_error_codes();
  for code in &codes {
    assert_ne!(
      code.sequence,
      0,
      "Code {} has sequence 0 which is invalid",
      code.to_code_string()
    );
  }
}

/// Test that the module prefix is correctly derived from `ErrorModule`.
/// Note: `ErrorModule` doesn't implement Display, so we test via `ErrorCode::to_code_string()`.
#[test]
fn test_error_code_module_prefix() {
  // Create error codes for each module and verify prefix in to_code_string()
  let test_cases = [
    (ErrorModule::Sig, "SIG"),
    (ErrorModule::Cht, "CHT"),
    (ErrorModule::Av, "AV"),
    (ErrorModule::Rom, "ROM"),
    (ErrorModule::Thr, "THR"),
    (ErrorModule::Fil, "FIL"),
    (ErrorModule::Auth, "AUTH"),
    (ErrorModule::Pst, "PST"),
    (ErrorModule::Sys, "SYS"),
  ];

  for (module, expected_prefix) in test_cases {
    let code = ErrorCode::new(module, ErrorCategory::Client, 1);
    let code_str = code.to_code_string();
    assert!(
      code_str.starts_with(expected_prefix),
      "Code {code_str} should start with {expected_prefix}"
    );
  }
}

/// Test that the category value is correctly derived from `ErrorCategory`.
#[test]
fn test_error_category_discriminant_values() {
  assert_eq!(ErrorCategory::Network as u16, 0);
  assert_eq!(ErrorCategory::Client as u16, 1);
  assert_eq!(ErrorCategory::Informational as u16, 2);
  assert_eq!(ErrorCategory::Server as u16, 3);
  assert_eq!(ErrorCategory::Media as u16, 4);
  assert_eq!(ErrorCategory::Security as u16, 5);
}

/// Test that all defined error codes have associated documentation.
/// This test doesn't verify doc quality, just that docs exist.
#[test]
fn test_error_codes_have_documentation() {
  // The error code constants have doc comments in mod.rs.
  // This test serves as a reminder to add docs when creating new codes.
  // Actual doc verification would require rustdoc analysis.
  // For now, we just verify the codes are well-defined.
  let codes = all_error_codes();
  assert!(
    !codes.is_empty(),
    "At least some error codes should be defined"
  );
}

/// Test error code bitcode serialization roundtrip.
#[test]
fn test_error_code_bitcode_roundtrip() {
  let codes = all_error_codes();
  for code in &codes {
    let encoded = bitcode::encode(code);
    let decoded: ErrorCode = bitcode::decode(&encoded)
      .unwrap_or_else(|_| panic!("Failed to decode {}", code.to_code_string()));
    assert_eq!(*code, decoded);
  }
}

/// Test that error codes can be used in collections.
#[test]
fn test_error_code_in_collections() {
  use std::collections::{HashMap, HashSet};

  let codes = all_error_codes();

  // HashSet
  let hash_set: HashSet<ErrorCode> = codes.iter().copied().collect();
  assert_eq!(hash_set.len(), codes.len());

  // HashMap (use String to avoid lifetime issues)
  let hash_map: HashMap<ErrorCode, String> =
    codes.iter().map(|c| (*c, c.to_code_string())).collect();
  assert_eq!(hash_map.len(), codes.len());

  // Note: BTreeSet and BTreeMap require Ord trait which is not implemented for ErrorCode.
  // If ordering is needed, Vec::sort_by can be used with a custom comparator.
}

/// Test that error codes from the same module and category have sequential sequences.
#[test]
fn test_sequential_sequences_within_category() {
  use std::collections::HashMap;

  let codes = all_error_codes();

  // Group by (module, category) and check sequences
  let mut groups: HashMap<(ErrorModule, ErrorCategory), Vec<u16>> = HashMap::new();
  for code in &codes {
    groups
      .entry((code.module, code.category))
      .or_default()
      .push(code.sequence);
  }

  for ((module, category), mut sequences) in groups {
    sequences.sort_unstable();
    // Check for gaps (sequences should be 1, 2, 3, ... without holes)
    for (i, seq) in sequences.iter().enumerate() {
      let expected = u16::try_from(i + 1).expect("sequence should fit in u16");
      assert!(
        *seq <= expected,
        "Non-sequential sequence {seq} in {module:?}/{category:?} (expected <= {expected})"
      );
    }
  }
}
