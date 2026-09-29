class VoteService
  def call(poll)
    @votes = []
    poll.votes.each { |v| @votes << v }
    #    ^ d: none
    #                      ^ d: app/vote_service.rb:3
  end
end
