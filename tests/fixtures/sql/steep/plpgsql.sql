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
  --                                                   ^ d: steep/schema.sql:3
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
-- #1243's review: a utility statement, an output label and a conflict target name no variable;
-- an INTO or loop target stays the variable under use_column; END LOOP, END CASE and a quoted "begin" keep scopes.
CREATE FUNCTION refill(id integer) RETURNS integer LANGUAGE plpgsql AS $$
DECLARE
  brim text;
  sips integer;
BEGIN
  INSERT INTO samovars (id, brim) VALUES (id, brim)
  ON CONFLICT (brim) DO UPDATE SET brim = EXCLUDED.brim;
  --           ^ d: steep/schema.sql:5
  --                               ^ d: steep/schema.sql:5
  CREATE INDEX ON samovars (brim);
  --                        ^ d: steep/schema.sql:5
  SELECT count(*) AS sips INTO sips FROM samovars;
  --                 ^ d: !jump
  --                           ^ d: steep/plpgsql.sql:75
  FOR i IN 1..3 LOOP
    sips := sips + i;
  END LOOP;
  RETURN sips;
  --     ^ d: steep/plpgsql.sql:75
END $$;
CREATE FUNCTION topple(spout integer) RETURNS integer LANGUAGE plpgsql AS $$
#variable_conflict use_column
DECLARE
  n integer;
BEGIN
  SELECT id, spout INTO n, spout FROM samovars;
  --         ^ d: steep/schema.sql:4
  --                       ^ d: steep/plpgsql.sql:92
  FOR spout IN SELECT id FROM samovars LOOP
  --  ^ d: steep/plpgsql.sql:92
    n := spout;
  END LOOP;
  RETURN spout;
  --     ^ d: steep/plpgsql.sql:92
END $$;
CREATE FUNCTION ladle(spout integer) RETURNS void LANGUAGE plpgsql AS $$
BEGIN
  INSERT INTO samovars (SELECT 1, spout, 'x');
  --                              ^ d: steep/plpgsql.sql:107
  UPDATE samovars SET brim = 'x', spout = spout + 1;
  --                              ^ d: steep/schema.sql:4
END $$;
CREATE FUNCTION stamp_it() RETURNS integer LANGUAGE plpgsql AS $$
BEGIN
  DECLARE
    stamp integer;
  BEGIN
    SELECT "begin" INTO stamp FROM samovars;
  END;
  DECLARE
    froth integer;
  BEGIN
    CASE froth WHEN 0 THEN froth := 1; ELSE NULL; END CASE;
  END;
  RETURN stamp + froth;
  --     ^ d: !jump
  --             ^ d: !jump
END $$;
-- A plpgsql body followed by a LANGUAGE sql function keeps its language; RETURNS TABLE names are not seen in LANGUAGE sql.
CREATE FUNCTION settle() RETURNS integer LANGUAGE plpgsql AS $$
DECLARE
  silt integer;
BEGIN
  RETURN silt;
  --     ^ d: steep/plpgsql.sql:133
END $$;
CREATE FUNCTION skim() RETURNS TABLE (dregs_out integer) LANGUAGE sql AS $$
  SELECT s.spout FROM samovars s JOIN urns u ON u.id = s.id ORDER BY dregs_out;
  --                                                                 ^ d: !jump
$$;
-- #1240: the UPDATE target and its alias qualify a column, as a FROM item does.
CREATE FUNCTION steep_out(spout integer) RETURNS void LANGUAGE plpgsql AS $$
BEGIN
  UPDATE samovars AS s SET brim = 'x' WHERE s.spout = spout;
  --                                          ^ d: steep/schema.sql:4
  --                                                  ^ d: steep/plpgsql.sql:143
  UPDATE samovars s SET brim = 'x' FROM urns u WHERE s.id = u.handle;
  --                                                   ^ d: steep/schema.sql:3
  --                                                          ^ d: steep/schema.sql:9
END $$;
UPDATE samovars SET spout = 1 WHERE samovars.id = 1;
--                                           ^ d: steep/schema.sql:3
-- #1245: a parenthesised SET list, a subscripted or field target and MERGE's UPDATE SET and INSERT (…) name no variable.
CREATE FUNCTION steep_more(spout integer, brim text) RETURNS void LANGUAGE plpgsql AS $$
BEGIN
  UPDATE samovars SET (id, spout) = (1, spout);
  --                       ^ d: steep/schema.sql:4
  --                                    ^ d: steep/plpgsql.sql:155
  UPDATE samovars SET brim = 1, (id, spout) = ROW(2, spout) WHERE id = 3;
  --                                 ^ d: steep/schema.sql:4
  --                                                 ^ d: steep/plpgsql.sql:155
  UPDATE samovars SET brim[1] = brim, brim.lid = brim;
  --                  ^ d: steep/schema.sql:5
  --                            ^ d: steep/plpgsql.sql:155
  --                                  ^ d: steep/schema.sql:5
  --                                             ^ d: steep/plpgsql.sql:155
  MERGE INTO samovars s USING samovars t ON s.id = t.id
    WHEN MATCHED THEN UPDATE SET spout = spout, (id, brim) = (1, brim)
  --                               ^ d: steep/schema.sql:4
  --                                       ^ d: steep/plpgsql.sql:155
  --                                                   ^ d: steep/schema.sql:5
  --                                                               ^ d: steep/plpgsql.sql:155
    WHEN NOT MATCHED THEN INSERT (spout, brim) VALUES (t.spout, brim);
  --                                ^ d: steep/schema.sql:4
  --                                       ^ d: steep/schema.sql:5
  --                                                              ^ d: steep/plpgsql.sql:155
  UPDATE samovars SET brim = coalesce(brim, spout);
  --                                  ^ d: steep/plpgsql.sql:155
  --                                        ^ d: steep/plpgsql.sql:155
END $$;
-- Outside plpgsql: a field target, MERGE's lists, the table named before a SET field, and an alias a subquery or CTE binds again.
UPDATE samovars SET brim.lid = 1;
--                  ^ d: steep/schema.sql:5
MERGE INTO samovars s USING urns u ON s.id = u.id WHEN NOT MATCHED THEN INSERT (spout) VALUES (u.handle);
--                                                                              ^ d: steep/schema.sql:4
MERGE INTO samovars s USING urns u ON s.id = u.id WHEN MATCHED THEN UPDATE SET brim = u.brim;
--                                                                             ^ d: steep/schema.sql:5
UPDATE tins SET tins.seal = 1;
--              ^ d: steep/schema.sql:36
UPDATE samovars s SET brim = 'x' RETURNING s.spout;
--                                           ^ d: steep/schema.sql:4
UPDATE samovars s SET brim = (SELECT u.brim FROM urns u WHERE u.id = s.id);
--                                                                     ^ d: steep/schema.sql:3
UPDATE samovars s SET spout = (SELECT max(s.handle) FROM urns s);
--                                          ^ d: steep/schema.sql:9
WITH moved AS (UPDATE samovars s SET spout = 0 RETURNING s.id) SELECT s.handle FROM urns s;
--                                                                       ^ d: steep/schema.sql:9
