class ApplicationController < ActionController::Base
  before_action :authenticate

  def authenticate
    @current_user = User.find(1)
  end
end
