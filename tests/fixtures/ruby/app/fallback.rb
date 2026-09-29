class Fallback
  def mention(account)
    TextFormatter.link_to_mention(account)
    #             ^ d: app/text_formatter.rb:3
  end
end
