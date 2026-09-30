# A concern's `included do` scope is a class method of the class that includes it (#374).
module Suspensions
  extend ActiveSupport::Concern

  included do
    scope :suspended, -> { where.not(suspended_at: nil) }
  end
end
