# A reopened controller whose second `before_action` assigns the same `@ivar` keeps the picker
# (#1285).
class PinsController < BaseController
  before_action :set_pin

  def show
    @pin
    #^ d: picker app/pins_controller.rb:14, app/pins_reopened.rb:5
  end

  private

  def set_pin
    @pin = Pin.find(params[:id])
  end
end
