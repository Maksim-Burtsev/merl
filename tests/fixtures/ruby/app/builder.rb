# #390: with the core and the gems unread, the one namesake of the project is offered.
class Builder
  def build(logger)
    logger.info { 'Build: building assets' }
    #      ^ d: picker app/worker_batch.rb:2
    #      status: info: by name, 1 match
    logger&.info
    #       ^ d: picker app/worker_batch.rb:2
    WorkerBatch.new.info
    #               ^ d: picker app/worker_batch.rb:2
    WorkerBatch.drain
    #           ^ d: app/worker_batch.rb:6
  end

  def assets
    []
  end

  def run
    assets + self.assets
    # ^ d: app/builder.rb:15
    #             ^ d: app/builder.rb:15
  end
end
