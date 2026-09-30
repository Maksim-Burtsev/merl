class AdminController
  def show
    user = User.find(1)
    user
  end

  def reset
    nil
  end

  # Namesakes of what SessionsController reaches through its module and its superclass.
  def throttle(user)
    user
  end

  def authenticate!
    false
  end
end
