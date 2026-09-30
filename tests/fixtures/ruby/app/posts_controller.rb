# An `@ivar` of the class's chain: the superclass and the included module assign them (#521).
class PostsController < ApplicationController
  include Trackable

  def index
    @tracker.log(@current_user)
    #^ d: app/trackable.rb:3
    #             ^ d: app/application_controller.rb:5
  end
end
