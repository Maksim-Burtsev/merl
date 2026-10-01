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
    #    ^ d: picker app/post.rb:10
    post.title == "x"
    #    ^ d: picker app/post.rb:4
  end

  # The Rails DSL and the columns of `db/schema.rb` (#374). A member on a value found by name is
  # offered, not jumped to (#390); `Const.scope` is the class's, and jumps.
  def call(account, collection)
    account.followers.without_suspended.first
    #       ^ d: picker app/account.rb:5
    #                 ^ d: picker app/account.rb:12
    account.email
    #       ^ d: picker app/account.rb:13
    collection.language
    #          ^ d: picker db/schema.rb:4
    #          status: language: by name, 1 match
    collection.summary
    #          ^ d: picker db/schema.rb:5, db/schema.rb:10
    Account.without_suspended
    #       ^ d: app/account.rb:12
    Account.suspended
    #       ^ d: app/suspensions.rb:6
    collection.curator
    #          ^ d: picker app/collection.rb:3
    collection.cover
    #          ^ d: picker app/collection.rb:4
    collection.tags
    #          ^ d: picker app/collection.rb:5
    collection.pinned
    #          ^ d: picker app/collection.rb:6
    collection.lang
    #          ^ d: picker app/collection.rb:7
    collection.layout
    #          ^ d: picker app/collection.rb:8
    collection.settings
    #          ^ d: none
    collection.state
    #          ^ d: picker app/collection.rb:9
    collection.visibility
    #          ^ d: picker app/collection.rb:10
    collection.locale
    #          ^ d: none
    collection.handle
    #          ^ d: none
    collection.time_zone
    #          ^ d: picker app/collection.rb:16
    collection.email?
    #          ^ d: picker app/collection.rb:16
    Collection.recent
    #          ^ d: none
  end
end
