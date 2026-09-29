# The scopes of #383: a local is its method's, an `@ivar` its class's, and neither is a
# member behind a dot.
class SanitizeConfig
  def transform(node)
    scheme = node["href"]
    node["rel"] = scheme
    #             ^ d: app/sanitize_config.rb:5
  end

  def untransform
    node["rel"] = scheme
    #             ^ d: none
  end

  def widths(rows)
    rows.each do |row|
      width = row.size
      row.pad(width)
      #       ^ d: app/sanitize_config.rb:17
    end
    rows.pad(width)
    #        ^ d: none
  end
end
