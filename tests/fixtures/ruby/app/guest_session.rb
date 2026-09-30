# A namesake the chains above never reach. With every parent the project's, an `@ivar` no class
# of the chain assigns has no definition; with one that is not, any class's may be it (#521).
class GuestSession
  def sign_in
    @current_user = nil
    @tracker
    #^ d: none
  end
end

class LegacyReport < ActiveRecord::Base
  def owner
    @current_user
    #^ d: picker app/application_controller.rb:5, app/guest_session.rb:5
  end
end
