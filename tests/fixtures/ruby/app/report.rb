class Report
  def targets
    Account.remote.where(id: 1)
    #       ^ d: app/account.rb:4
  end

  def remotes(account)
    account.remote?
    #       ^ d: picker app/account.rb:7
  end
end
