SELECT * FROM strainers;
--            ^ d: migrations/2024-02-01-000002_widen_strainers/up.sql:3
CREATE FUNCTION drain_strainers () RETURNS trigger LANGUAGE sql AS 'SELECT 1';
CREATE TRIGGER drain_strainers AFTER DELETE ON strainers FOR EACH ROW EXECUTE FUNCTION drain ();
