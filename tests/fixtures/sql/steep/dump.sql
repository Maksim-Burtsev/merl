-- #1227: a lone /* inside a comment leaves nesting open to the end, so it reads as MySQL.
/* Nightly backups go to /var/backups/* on the host. */
CREATE TABLE scones (id int);
SELECT id FROM scones;
--     ^ d: steep/dump.sql:3
