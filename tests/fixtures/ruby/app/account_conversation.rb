class AccountConversation
  def remove_status(recipient, status)
    participants_from_status(recipient, status)
    #^ d: app/account_conversation.rb:9
  end

  private

  def participants_from_status(recipient, status)
    [recipient, status]
  end
end
