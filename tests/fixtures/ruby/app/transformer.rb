class Transformer
  def must_not(clauses)
    clauses.fetch(:must_not, [])
    #       ^ d: none
  end

  def fresh(manifest)
    manifest.fetch? || manifest.fetch!
    #        ^ d: picker app/manifest.rb:2
    #                           ^ d: picker app/manifest.rb:6
    manifest.fetch!=1
    #        ^ d: none
  end
end
