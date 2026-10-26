# An `@ivar` an action reads but never assigns is the one `before_action` callback that assigns
# it; any doubt keeps the picker (#1285).
class HooksController < BaseController
  before_action :set_hook, except: [:index, :new, :create]

  def new
    @hook = Hook.new
  end

  def create
    @hook = Hook.new(params)
    @hook.save
    #^ d: picker app/hooks_controller.rb:7, app/hooks_controller.rb:11, app/hooks_controller.rb:30
  end

  def disable
    @hook.disable!
    #^ d: app/hooks_controller.rb:30
    # status: @hook
  end

  def index
    @hook
    #^ d: picker app/hooks_controller.rb:7, app/hooks_controller.rb:11, app/hooks_controller.rb:30
  end

  private

  def set_hook
    @hook = Hook.find(params[:id])
  end

  def helper
    @hook
    #^ d: picker app/hooks_controller.rb:7, app/hooks_controller.rb:11, app/hooks_controller.rb:30
  end
end

class TokensController < BaseController
  before_action :set_token, only: :show
  before_action :load_token, only: [:show, :edit]

  def show
    @token
    #^ d: picker app/hooks_controller.rb:56, app/hooks_controller.rb:60
  end

  def edit
    @token
    #^ d: app/hooks_controller.rb:60
  end

  private

  def set_token
    @token = Token.find(params[:id])
  end

  def load_token
    @token = Token.find_by(slug: params[:slug])
  end
end

class BadgesController < BaseController
  before_action :set_badge
  before_action do
    @badge = Badge.first
  end

  def show
    @badge
    #^ d: picker app/hooks_controller.rb:67, app/hooks_controller.rb:78
  end

  private

  def set_badge
    @badge = Badge.find(params[:id])
  end
end

class StampsController < BaseController
  before_action :set_stamp

  def show
    @stamp
    #^ d: picker app/hooks_controller.rb:93, app/hooks_controller.rb:97
  end

  private

  def set_stamp
    @stamp = Stamp.find(params[:id])
  end

  def build_stamp
    @stamp = Stamp.new
  end
end

class ShelvesController < BaseController
  before_action :set_shelf
  skip_before_action :set_shelf, only: :show

  def show
    @shelf
    #^ d: picker app/hooks_controller.rb:111, app/hooks_controller.rb:117
  end

  def new
    @shelf = Shelf.new
  end

  private

  def set_shelf
    @shelf = Shelf.find(params[:id])
  end
end

class CupsController < BaseController
  before_action { @cup = Cup.first }
  before_action :set_cup

  def show
    @cup
    #^ d: picker app/hooks_controller.rb:131, app/hooks_controller.rb:137
  end

  def new
    @cup = Cup.new
  end

  private

  def set_cup
    @cup = Cup.find(params[:id])
  end
end

class LeavesController < BaseController
  before_action :set_leaf,
    only: %i[show edit]

  def show
    @leaf
    #^ d: app/hooks_controller.rb:162
  end

  def destroy
    @leaf
    #^ d: picker app/hooks_controller.rb:156, app/hooks_controller.rb:162
  end

  def new
    @leaf = Leaf.new
  end

  private

  def set_leaf
    @leaf = Leaf.find(params[:id])
  end
end

class TwigsController < BaseController
  before_action(
    :set_twig,
    except: :index,
  )

  def index
    @twig
    #^ d: picker app/hooks_controller.rb:183, app/hooks_controller.rb:189
  end

  def show
    @twig
    #^ d: app/hooks_controller.rb:189
  end

  def new
    @twig = Twig.new
  end

  private

  def set_twig
    @twig = Twig.find(params[:id])
  end
end
