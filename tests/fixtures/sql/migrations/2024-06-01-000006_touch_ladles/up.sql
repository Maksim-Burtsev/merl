-- A trigger is per table and a function per signature.
CREATE TRIGGER touch AFTER UPDATE ON ladles FOR EACH ROW EXECUTE FUNCTION drain ();
CREATE FUNCTION scoop (n int) RETURNS int LANGUAGE sql AS 'SELECT 1';
