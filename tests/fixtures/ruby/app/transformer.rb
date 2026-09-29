class Transformer
  def must_not(clauses)
    clauses.fetch(:must_not, [])
    #       ^ d: none
  end

  def fresh(manifest)
    manifest.fetch? || manifest.fetch!
    #        ^ d: app/manifest.rb:2
    #                           ^ d: app/manifest.rb:6
  end
end
