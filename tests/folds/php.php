<?php
// Every construct `f` folds in PHP, and the cases that once broke it.

namespace App\Services;

use App\Models\Song;  // f: 6-12
use Illuminate\Support\{  // f: 6-12
    Collection,  // f: 6-12
    Str  // f: 6-12
};  // f: 6-12

use Throwable;  // f: 6-12

#[Attribute]  // f: 14-99
final class SongService extends Service implements Contract  // f: 14-99
{  // f: 14-99
    use Queueable;  // f: 14-99

    private const TYPES = [  // f: 19-22
        'mp3',  // f: 14-99
        'flac',  // f: 14-99
    ];  // f: 14-99

    #[Route('/songs')]  // f: 24-94
    public static function find(  // f: 24-94
        int $id,  // f: 24-94
        array $options = []  // f: 24-94
    ): ?Song {  // f: 24-94
        static $cache = [  // f: 29-31
            'a' => 1,  // f: 24-94
        ];  // f: 24-94
        $text = 'a { brace  // f: 24-94
            spanning lines';  // f: 24-94
        $doc = <<<EOT  // f: 24-94
            { not a block  // f: 24-94
            EOT;  // f: 24-94
        # a hash comment {  // f: 24-94
        if ($id > 0) {  // f: 38-46
            return null;  // f: 24-94
        } elseif ($id < 0) {  // f: 24-94
            return null;  // f: 24-94
        } else if ($id === 0) {  // f: 42-46
            return null;  // f: 24-94
        } else {  // f: 24-94
            $id = 1;  // f: 24-94
        }  // f: 24-94
        foreach ($options as $key => $value) {  // f: 47-49
            echo $value;  // f: 24-94
        }  // f: 24-94
        for ($i = 0; $i < 3; $i++) {  // f: 50-52
            echo $i;  // f: 24-94
        }  // f: 24-94
        while ($id > 0) {  // f: 53-55
            $id--;  // f: 24-94
        }  // f: 24-94
        do {  // f: 56-58
            $id++;  // f: 24-94
        } while ($id < 3);  // f: 24-94
        switch ($id) {  // f: 59-64
            case 1:  // f: 24-94
                break;  // f: 24-94
            default:  // f: 24-94
                break;  // f: 24-94
        }  // f: 24-94
        try {  // f: 65-71
            work();  // f: 24-94
        } catch (Throwable $e) {  // f: 24-94
            report($e);  // f: 24-94
        } finally {  // f: 24-94
            close();  // f: 24-94
        }  // f: 24-94
        $kind = match ($id) {  // f: 72-75
            1 => 'one',  // f: 24-94
            default => 'many',  // f: 24-94
        };  // f: 24-94
        $list = array(  // f: 76-79
            1,  // f: 24-94
            2  // f: 24-94
        );  // f: 24-94
        $row = (object) [  // f: 80-82
            'id' => $id,  // f: 24-94
        ];  // f: 24-94
        $value = $options[  // f: 24-94
            'key'  // f: 24-94
        ];  // f: 24-94
        $call = collect($options)->map(function ($o) {  // f: 24-94
            return $o;  // f: 24-94
        });  // f: 24-94
        if ($id):  // f: 89-91
            echo 'alt';  // f: 24-94
        endif;  // f: 24-94
        return $options['song']  // f: 24-94
            ?? null;  // f: 24-94
    }  // f: 24-94

    abstract protected function make(  // f: 96-98
        int $id  // f: 96-98
    ): Song;  // f: 96-98
}  // f: 14-99

interface Contract  // f: 101-104
{  // f: 101-104
    public function find(int $id): ?Song;  // f: 101-104
}  // f: 101-104

function helper(): void  // f: 106-109
{  // f: 106-109
    echo Song::class;  // f: 106-109
}  // f: 106-109
