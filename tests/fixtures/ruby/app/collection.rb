# The Rails DSL declares the name it is given (#374).
class Collection < ApplicationRecord
  belongs_to :curator
  has_one_attached :cover
  has_and_belongs_to_many :tags
  attribute :pinned
  alias_attribute :lang, :language
  store_accessor :settings, :theme, :layout
  enum :state, { draft: 0, live: 1 }
  enum visibility: { open: 0, closed: 1 }
  validates :language, presence: true
  delegate :locale, to: :curator, prefix: true
  delegate :handle,
           to: :curator,
           prefix: :curator
  delegate :time_zone, :email?, to: :curator, allow_nil: true

  SAMPLE = <<~RUBY
    has_many :followers
    scope :recent, -> { all }
  RUBY
end
