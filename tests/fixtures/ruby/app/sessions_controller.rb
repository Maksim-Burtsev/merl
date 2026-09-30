# The bindings of #365: parameters, block parameters and locals, and a bare call read in the
# class of `self` first.
class SessionsController < BaseController
  include Throttled

  def on_success(user, measure = nil, *rest, strategy:, remember: false, **opts, &done)
    user.update_sign_in!(measure)
    #^ d: app/sessions_controller.rb:6
    #                    ^ d: app/sessions_controller.rb:6
    strategy.call(opts, done)
    #^ d: app/sessions_controller.rb:6
    #             ^ d: app/sessions_controller.rb:6
    #                   ^ d: app/sessions_controller.rb:6
    remember && rest
    #^ d: app/sessions_controller.rb:6
    #           ^ d: app/sessions_controller.rb:6
    rest.each do |(first, second), idx|
      first + second + idx
      #^ d: app/sessions_controller.rb:17
      #       ^ d: app/sessions_controller.rb:17
      #                ^ d: app/sessions_controller.rb:17
      puts user
      #    ^ d: app/sessions_controller.rb:6
    end
    throttle(user)
    #^ d: app/throttled.rb:2
    authenticate!
    #^ d: app/base_controller.rb:2
    self.audit(user)
    #    ^ d: app/sessions_controller.rb:51
    reset
    #^ d: app/sessions_controller.rb:55
  end

  def recover
    retries = 0
    begin
      retries += 1
    rescue IOError => e
      puts e.message
      #    ^ d: app/sessions_controller.rb:39
      puts retries
      #    ^ d: app/sessions_controller.rb:36
    end
    for attempt in 1..3
      puts attempt
      #    ^ d: app/sessions_controller.rb:45
    end
  end

  def audit(user)
    user
  end

  def reset
    nil
  end

  class << self
    def reset
      nil
    end

    def reset_all
      reset
      #^ d: app/sessions_controller.rb:60
    end
  end
end
