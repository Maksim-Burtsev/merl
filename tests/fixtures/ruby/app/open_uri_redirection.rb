module OpenURI
  def self.redirectable?(uri1, uri2)
    uri1.scheme.casecmp(uri2.scheme).zero?
    #                        ^ d: none
  end
end
