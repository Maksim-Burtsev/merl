class Bulletin
end

class BulletinWorker
  include Mailers
end

class Digest
  def send_all
    Bulletin.deliver_all
    #        ^ d: none
    BulletinWorker.deliver_all
    #              ^ d: app/mailers.rb:5
  end
end
