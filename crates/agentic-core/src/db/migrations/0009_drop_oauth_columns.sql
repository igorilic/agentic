-- Migration 0009: drop OAuth-specific columns from auth_accounts.
-- Stages 1+2 of the auth refactor (commits 0e1b1f6, ffc4124, 3839fa4)
-- removed all OAuth flows. The `client_id` (GHES BYO OAuth app) and
-- `token_expires_at` (OAuth refresh) columns are no longer written to.
-- Drop them to keep the schema honest.
--
-- SQLite supports DROP COLUMN since 3.35 (2021). The bundled rusqlite
-- in this workspace ships SQLite >= 3.35, so DROP COLUMN works directly.

ALTER TABLE auth_accounts DROP COLUMN client_id;
ALTER TABLE auth_accounts DROP COLUMN token_expires_at;
