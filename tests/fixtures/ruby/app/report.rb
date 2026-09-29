class Report
  def targets
    Account.remote.where(id: 1)
    #       ^ d: none; want app/account.rb:4 (#374)
  end

  def remotes(account)
    account.remote?
    #       ^ d: app/account.rb:7
  end
end
