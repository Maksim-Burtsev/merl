class VerifyQuoteService
  def call(quote, request_id: nil)
    @request_id = request_id
    #^ d: app/verify_quote_service.rb:3
    #             ^ d: none; want app/verify_quote_service.rb:2 (#365)
  end
end

class FetchService
  def call(uri, request_id: nil)
    @request_id = request_id
  end
end
