# The class reopened: its `@votes` is the one of vote_service.rb.
class VoteService
  def tally
    @votes.size
    #^ d: app/vote_service.rb:3
  end
end
