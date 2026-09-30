# The status names a method's parameter, a block's parameter and a local by the method, as
# Python's are (#526).
class RefundsController < BaseController
  def on_refund(order, amount)
    total = amount
    order.each do |entry|
      puts order
      #    ^ d: app/refunds_controller.rb:4
      #      status: RefundsController.on_refund.order (local)
      puts entry
      #    ^ d: app/refunds_controller.rb:6
      #      status: RefundsController.on_refund.entry (local)
      puts total
      #    ^ d: app/refunds_controller.rb:5
      #      status: RefundsController.on_refund.total (local)
    end
  end

  def self.refund_all(orders)
    orders
    #^ d: app/refunds_controller.rb:19
    # status: RefundsController.refund_all.orders (local)
  end
end
