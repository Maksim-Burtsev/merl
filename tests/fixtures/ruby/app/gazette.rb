# A concern included under a modifier is included all the same (#793).
class Gazette
  include Mailers if ENV["MAIL"]
end

class Newsroom
  def publish
    Gazette.deliver_all
    #       ^ d: app/mailers.rb:5
  end
end
