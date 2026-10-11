# A parent named like the class is the outer one, never the class itself (#793).
module Admin
  class Roster < Roster
    def show
      headcount
      # ^ d: app/roster.rb:2
      @seats
      # ^ d: app/roster.rb:5
    end
  end
end
