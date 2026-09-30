module Shop
  class Order < ApplicationRecord
    has_many :lines
    scope :recent, -> { where(fresh: true) }
    delegate :rate, to: :tariff

    def recent?
      true
    end
  end

  # Settles the order, since `update_all``
  # runs no callbacks
  class Ledger
    def settle!
      prepare!
    end

    def prepare!
      true
    end
  end
end
