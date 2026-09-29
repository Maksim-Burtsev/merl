# Ruby's own literals (#379): a heredoc, a `=begin` block and the data after `__END__` declare
# nothing; a `#` comment ends with its line, whatever quote it holds.
module Shop
  class Texts
    def listing
      join("#", <<~A, <<-'B') # two heredocs opened on one line, read in order
        def phantom_a
      A
        def phantom_b
        B
    end

    def plain
      <<SQL
  SQL
def phantom_c
SQL
    end

    def after
      listing
#     ^ d: lib/shop/texts.rb:5
      phantom_a
#     ^ d: none
      phantom_b
#     ^ d: none
      phantom_c
#     ^ d: none
      phantom_d
#     ^ d: none
      plain
#     ^ d: lib/shop/texts.rb:13
    end
  end
end

__END__
def phantom_d
