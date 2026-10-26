class PinsController
  before_action :load_pin

  def load_pin
    @pin = Pin.find_by(slug: params[:slug])
  end
end
