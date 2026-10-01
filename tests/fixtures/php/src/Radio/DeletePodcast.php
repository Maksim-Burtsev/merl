<?php

namespace Shop\Radio;

class DeletePodcast
{
    private Archive $archive;

    public function handle(UserUnsubscribed $event): void
    {
        $event->podcast->subscribers();
        //      ^ d: src/Radio/UserUnsubscribed.php:8
        //       status: via $event: UserUnsubscribed
        //               ^ d: src/Radio/Podcast.php:7
        //                status: via $event: UserUnsubscribed → podcast: Podcast
        $info = new ScanInformation();
        $info->toArray();
        //     ^ d: src/Radio/ScanInformation.php:7
        //      status: via $info: ScanInformation
        $this->archive->find();
        //              ^ d: src/Radio/Archive.php:9
        //               status: via $this->archive: Archive
        $this->archive->find()->subscribers();
        //                      ^ d: picker src/Radio/Archive.php:14, src/Radio/Podcast.php:7
        $made = ScanInformation::make();
        $made->toArray();
        //     ^ d: src/Radio/ScanInformation.php:7
        //      status: via ScanInformation::make(): self
        $scanned = $this->scan();
        $scanned->toArray();
        //        ^ d: src/Radio/ScanInformation.php:7
        //         status: via $this->scan(): ScanInformation
        $brand = radio_branding();
        $brand->toArray();
        //      ^ d: src/Radio/Branding.php:7
        //       status: via radio_branding(): Branding
        $own = new self();
        $own->archive->subscribers();
        //             ^ d: src/Radio/Archive.php:14
        //              status: via $own: DeletePodcast → archive: Archive
    }

    public function wrapped(
        ?ScanInformation $maybe,
        Branding $brand,
    ): void {
        $maybe->toArray();
        //      ^ d: src/Radio/ScanInformation.php:7
        //       status: via $maybe: ScanInformation
        $brand->toArray();
        //      ^ d: src/Radio/Branding.php:7
        //       status: via $brand: Branding
    }

    public function refused(ScanInformation|Branding $either, mixed $any, ScanInformation&Branding $both, array $infos): void
    {
        $either->toArray();
        //       ^ d: picker src/Radio/Branding.php:7, src/Radio/ScanInformation.php:7
        $any->toArray();
        //    ^ d: picker src/Radio/Branding.php:7, src/Radio/ScanInformation.php:7
        $both->toArray();
        //     ^ d: picker src/Radio/Branding.php:7, src/Radio/ScanInformation.php:7
        $late = ScanInformation::fresh();
        $late->toArray();
        //     ^ d: picker src/Radio/Branding.php:7, src/Radio/ScanInformation.php:7
        /** @var ScanInformation $doc */
        $doc = $infos[0];
        $doc->toArray();
        //    ^ d: picker src/Radio/Branding.php:7, src/Radio/ScanInformation.php:7
        foreach ($infos as $one) {
            $one->toArray();
            //    ^ d: picker src/Radio/Branding.php:7, src/Radio/ScanInformation.php:7
        }
        if ($infos) {
            $two = new ScanInformation();
        } else {
            $two = new Branding();
        }
        $two->toArray();
        //    ^ d: picker src/Radio/Branding.php:7, src/Radio/ScanInformation.php:7
        $guessed = $this->guess();
        $guessed->toArray();
        //        ^ d: picker src/Radio/Branding.php:7, src/Radio/ScanInformation.php:7
        $chained = ScanInformation::make()->toArray();
        $chained->toArray();
        //        ^ d: picker src/Radio/Branding.php:7, src/Radio/ScanInformation.php:7
    }

    private function scan(): ScanInformation
    {
        return new ScanInformation();
    }

    /** @return ScanInformation */
    private function guess()
    {
        return new ScanInformation();
    }

    public function ship(): bool
    {
        return Courier::depot();
        //     ^ d: src/Radio/Courier.php:5
        //              ^ d: src/Radio/Courier.php:7
        //               status: via Courier
    }
}
