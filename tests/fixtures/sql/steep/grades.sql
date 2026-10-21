-- A type named like a column of the one table: in a type position the name is the type.
CREATE TYPE grade AS ENUM ('first', 'second');

CREATE TABLE leaves (
  id integer,
  grade text
);

ALTER TABLE leaves ALTER COLUMN grade TYPE grade USING grade::grade;
--                              ^ d: steep/grades.sql:6
--                                         ^ d: steep/grades.sql:2
--                                                     ^ d: steep/grades.sql:6
--                                                            ^ d: steep/grades.sql:2

ALTER TABLE leaves ADD COLUMN sorting grade;
--                                    ^ d: steep/grades.sql:2

SELECT id::grade, grade FROM leaves;
--         ^ d: steep/grades.sql:2
--                ^ d: steep/grades.sql:6

SELECT CAST(id AS grade) FROM leaves;
--                ^ d: steep/grades.sql:2

UPDATE leaves SET grade = 'first'::grade WHERE id = 1;
--                ^ d: steep/grades.sql:6
--                                 ^ d: steep/grades.sql:2
