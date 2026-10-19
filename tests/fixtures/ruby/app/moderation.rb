# A bare constant resolves through the nesting, then the ancestors, then the top level (#1095).
module Moderation
  class Account
  end

  class Queue
    def accounts
      Account.all
#     ^ d: app/moderation.rb:3
    end
  end
end

class Moderation::Sweep
  def accounts
    Account.all
#   ^ d: app/account.rb:3
  end
end

class Moderation::Base
  class Account
  end
end

class Moderation::Desk < Moderation::Base
  def accounts
    Account.all
#   ^ d: app/moderation.rb:22
  end
end
