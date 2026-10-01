# The columns of `db/schema.rb` declare what a model reads (#374): one candidate per table.
ActiveRecord::Schema[7.1].define(version: 2026_09_01_000000) do
  create_table "collections", force: :cascade do |t|
    t.string "language"
    t.text "summary"
    t.timestamps
  end

  create_table "statuses", force: :cascade do |t|
    t.text "summary"
    t.index ["summary"], name: "index_statuses_on_summary"
  end
end
