module Searchable
  extend ActiveSupport::Concern

  class_methods do
    def search_by(query)
      query
    end
  end

  module ClassMethods
    def reindex
      true
    end
  end

  def search_by(query)
    query
  end

  def reindex
    false
  end
end
