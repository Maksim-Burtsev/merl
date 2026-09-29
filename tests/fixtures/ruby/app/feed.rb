class Feed
  def load(post)
    Post.search_by("x")
    #    ^ d: app/searchable.rb:5
    Post.reindex
    #    ^ d: app/searchable.rb:11
    Post.new("t")
    #    ^ d: app/post.rb:6
    Post.find(1)
    #    ^ d: none
    Post.publish
    #    ^ d: none
    Helpers.slugify("x")
    #       ^ d: app/helpers.rb:4
    Paths.root
    #     ^ d: app/helpers.rb:12
    Clock.tick
    #     ^ d: app/helpers.rb:18
    post.title = "x"
    #    ^ d: app/post.rb:10
  end
end
