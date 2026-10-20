# A migration keeps its own copy of a model, out of the app's reach (#1095).
class BackfillAccountDomains < ActiveRecord::Migration[5.2]
  class Account < ApplicationRecord
  end

  def up
    Account.where(domain: '').update_all(domain: nil)
#   ^ d: db/migrate/20190101_backfill_account_domains.rb:3
  end
end
