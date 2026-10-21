-- #1225: in LANGUAGE sql a column wins over a parameter of its name; in plpgsql the parameter stays.
-- #1226: a dollar string inside a dollar body closes on its own tag, and the body goes on as code.
CREATE FUNCTION simmer(spout integer, leaves integer) RETURNS integer LANGUAGE sql AS $$
  SELECT spout + leaves FROM samovars
  --     ^ d: steep/schema.sql:4
  --             ^ d: steep/infuse.sql:3
$$;
CREATE FUNCTION stew(spout integer) RETURNS integer AS $$
BEGIN
  RETURN (SELECT spout FROM samovars);
  --             ^ d: steep/infuse.sql:8
END
$$ LANGUAGE plpgsql;
CREATE FUNCTION mull() RETURNS void AS $fn$
BEGIN
  EXECUTE $q$ SELECT 1 $q$;
  CREATE TABLE IF NOT EXISTS dregs (id integer);
END
$fn$ LANGUAGE plpgsql;
SELECT * FROM dregs;
--            ^ d: steep/infuse.sql:17
