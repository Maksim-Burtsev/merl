-- #1239: a plpgsql parameter yields to the column in an INSERT column list, an UPDATE SET target,
-- a qualified name and a body under #variable_conflict use_column.
CREATE FUNCTION brew(id integer, spout integer) RETURNS void LANGUAGE plpgsql AS $$
BEGIN
  INSERT INTO samovars (spout) VALUES (spout);
  --                    ^ d: steep/schema.sql:4
  --                                   ^ d: steep/plpgsql.sql:3
  UPDATE samovars SET spout = spout + 1 WHERE samovars.id = id;
  --                  ^ d: steep/schema.sql:4
  --                          ^ d: steep/plpgsql.sql:3
  --                                                   ^ d: none; want steep/schema.sql:3 (#1240)
  --                                                        ^ d: steep/plpgsql.sql:3
  PERFORM samovars.spout FROM samovars;
  --               ^ d: steep/schema.sql:4
END $$;
CREATE FUNCTION pour_off(spout integer) RETURNS integer AS $$
#variable_conflict use_column
BEGIN
  RETURN (SELECT spout FROM samovars);
  --             ^ d: steep/schema.sql:4
END
$$ LANGUAGE plpgsql;
-- #1234: a DECLARE variable answers as a parameter does, in its own block; two nested ones give the picker.
CREATE FUNCTION steep_for(n integer) RETURNS integer LANGUAGE plpgsql AS $$
DECLARE
  cups integer := n;
  brim text;
BEGIN
  SELECT count(*) INTO cups FROM samovars;
  --                   ^ d: steep/plpgsql.sql:26
  INSERT INTO samovars (brim) VALUES (brim);
  --                    ^ d: steep/schema.sql:5
  --                                  ^ d: steep/plpgsql.sql:27
  IF cups > 0 THEN
    DECLARE
      cups text;
      dregs_left integer;
    BEGIN
      RETURN CASE WHEN cups IS NULL THEN 0 ELSE 1 END;
      --               ^ d: picker steep/plpgsql.sql:26, steep/plpgsql.sql:36
    END;
  END IF;
  RETURN cups + dregs_left;
  --     ^ d: steep/plpgsql.sql:26
  --            ^ d: !jump
END $$;
CREATE FUNCTION spill() RETURNS integer AS $$
#variable_conflict use_column
DECLARE
  spout integer;
BEGIN
  RETURN (SELECT spout FROM samovars);
  --             ^ d: steep/schema.sql:4
END
$$ LANGUAGE plpgsql;
-- RETURNS TABLE names are output parameters, and a DO block is plpgsql too.
CREATE FUNCTION leftovers() RETURNS TABLE (handle_left integer) LANGUAGE plpgsql AS $$
BEGIN
  handle_left := 1;
  -- ^ d: steep/plpgsql.sql:57
  RETURN NEXT;
END $$;
DO $$
DECLARE
  tea_left integer;
BEGIN
  tea_left := 1;
  -- ^ d: steep/plpgsql.sql:65
END $$;
