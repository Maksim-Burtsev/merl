# With the cursor on the `@` of an instance or class variable, `d` reads the variable (#1098).
class Rosary
  @@beads = 0

  def initialize(rows)
    @rows = rows
  end

  def count
    @rows.size + @@beads
   #^ d: app/rosary.rb:6
   #             ^ d: app/rosary.rb:3
   #              ^ d: app/rosary.rb:3
   #               ^ d: app/rosary.rb:3
  end
end
