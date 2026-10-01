# A migration's column is history: `db/schema.rb` alone declares it (#374).
class AddLanguageToCollections < ActiveRecord::Migration[7.1]
  def change
    create_table :drafts do |t|
      t.string :language
    end
  end
end
