CREATE TRIGGER touch AFTER UPDATE ON strainers FOR EACH ROW EXECUTE FUNCTION drain ();
CREATE FUNCTION scoop (n int, m int) RETURNS int LANGUAGE sql AS 'SELECT 2';
CREATE OR REPLACE FUNCTION scoop (n int) RETURNS int LANGUAGE sql AS 'SELECT 3';
DROP TRIGGER touch ON ladles;
--           ^ d: picker migrations/2024-06-01-000006_touch_ladles/up.sql:2, migrations/2024-07-01-000007_touch_strainers/up.sql:1
SELECT scoop(1);
--     ^ d: picker migrations/2024-07-01-000007_touch_strainers/up.sql:2, migrations/2024-07-01-000007_touch_strainers/up.sql:3
