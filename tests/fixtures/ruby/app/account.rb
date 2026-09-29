# The Ruby names of #387: a `?`, `!` or `=` is part of the name, `Const.m` is a class method,
# `A::B` is a path.
class Account < ApplicationRecord
  scope :remote, -> { where.not(domain: nil) }
  has_many :followers

  def remote?
    domain.present?
  end
end
