# A concern's class methods count only for the classes that include it in their body (#793).
module Mailers
  extend ActiveSupport::Concern
  class_methods do
    def deliver_all = true
  end
end
