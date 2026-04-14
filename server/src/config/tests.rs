use super::*;

#[test]
fn test_log_rotation_from_str() {
  assert_eq!("daily".parse::<LogRotation>().unwrap(), LogRotation::Daily);
  assert_eq!("DAILY".parse::<LogRotation>().unwrap(), LogRotation::Daily);
  assert_eq!(
    "hourly".parse::<LogRotation>().unwrap(),
    LogRotation::Hourly
  );
  assert_eq!(
    "HOURLY".parse::<LogRotation>().unwrap(),
    LogRotation::Hourly
  );
  assert_eq!("never".parse::<LogRotation>().unwrap(), LogRotation::Never);
  assert_eq!("NEVER".parse::<LogRotation>().unwrap(), LogRotation::Never);
  assert_eq!(
    "invalid".parse::<LogRotation>().unwrap(),
    LogRotation::Daily
  );
}

#[test]
fn test_default_config() {
  let config = Config::default();
  assert_eq!(config.addr.port(), 3000);
  assert!(!config.jwt_secret.is_empty());
  assert!(!config.ice_servers.is_empty());
  assert!(config.tls.is_none());
  assert_eq!(config.log_rotation, LogRotation::Daily);
  assert_eq!(config.heartbeat_interval, Duration::from_secs(30));
  assert_eq!(config.heartbeat_timeout, Duration::from_secs(60));
  assert_eq!(config.send_queue_size, 256);
}
