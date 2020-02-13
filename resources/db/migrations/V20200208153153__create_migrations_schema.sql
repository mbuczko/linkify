CREATE TABLE IF NOT EXISTS migrations (
  version TEXT NOT NULL UNIQUE,
  description TEXT NOT NULL,
  script TEXT NOT NULL,
  run_at DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now', 'localtime')),
  app_semver TEXT NOT NULL
);
