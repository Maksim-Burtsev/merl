<p>Don't fold the page around the code.</p>
<style>#main { color: red; }</style>
<?php
// Every construct `f` folds in PHP, and the cases that once broke it.

namespace App\Services;

use App\Models\Song;  // f: 8-18
use function App\Support\{  // f: 8-18
    tap,  // f: 8-18
    when  // f: 8-18
};  // f: 8-18
use Illuminate\Support\{  // f: 8-18
    Collection,  // f: 8-18
    Str  // f: 8-18
};  // f: 8-18

use Throwable;  // f: 8-18

#[Attribute]  // f: 20-130
final class SongService extends Service implements Contract  // f: 20-130
{  // f: 20-130
    use Queueable;  // f: 20-130
    use Dispatchable;  // f: 20-130
    use Macroable;  // f: 20-130

    private const TYPES = [  // f: 27-30
        'mp3',  // f: 20-130
        'flac',  // f: 20-130
    ];  // f: 20-130

    #[Route('/songs')]  // f: 32-125
    public static function find(  // f: 32-125
        int $id,  // f: 32-125
        array $options = []  // f: 32-125
    ): ?Song {  // f: 32-125
        static $cache = [  // f: 37-39
            'a' => 1,  // f: 32-125
        ];  // f: 32-125
        $text = 'a { brace  // f: 32-125
            spanning lines';  // f: 32-125
        $doc = <<<EOT  // f: 32-125
            { not a block  // f: 32-125
            EOT;  // f: 32-125
        # a hash comment {  // f: 32-125
        if ($id > 0) {  // f: 46-54
            return null;  // f: 32-125
        } elseif ($id < 0) {  // f: 32-125
            return null;  // f: 32-125
        } else if ($id === 0) {  // f: 50-54
            return null;  // f: 32-125
        } else {  // f: 32-125
            $id = 1;  // f: 32-125
        }  // f: 32-125
        foreach ($options as $key => $value) {  // f: 55-57
            echo $value;  // f: 32-125
        }  // f: 32-125
        for ($i = 0; $i < 3; $i++) {  // f: 58-60
            echo $i;  // f: 32-125
        }  // f: 32-125
        while ($id > 0) {  // f: 61-63
            $id--;  // f: 32-125
        }  // f: 32-125
        do {  // f: 64-68
            $id++;  // f: 32-125
        } while (  // f: 32-125
            $id < 3  // f: 32-125
        );  // f: 32-125
        switch ($id) {  // f: 69-74
            case 1:  // f: 32-125
                break;  // f: 32-125
            default:  // f: 32-125
                break;  // f: 32-125
        }  // f: 32-125
        try {  // f: 75-81
            work();  // f: 32-125
        } catch (Throwable $e) {  // f: 32-125
            report($e);  // f: 32-125
        } finally {  // f: 32-125
            close();  // f: 32-125
        }  // f: 32-125
        $kind = match ($id) {  // f: 82-85
            1 => 'one',  // f: 32-125
            default => 'many',  // f: 32-125
        };  // f: 32-125
        $list = array(  // f: 86-89
            1,  // f: 32-125
            2  // f: 32-125
        );  // f: 32-125
        $row = (object) [  // f: 90-92
            'id' => $id,  // f: 32-125
        ];  // f: 32-125
        $value = $options[  // f: 32-125
            'key'  // f: 32-125
        ];  // f: 32-125
        $call = collect($options)->map(function ($o) {  // f: 32-125
            return $o;  // f: 32-125
        });  // f: 32-125
        if ($id):  // f: 99-104
            echo 'alt';  // f: 32-125
            if ($id > 1):  // f: 101-103
                echo 'nested';  // f: 32-125
            endif;  // f: 32-125
        endif;  // f: 32-125
        foreach ($options as $o):  // f: 105-107
            echo $o;  // f: 32-125
        endforeach;  // f: 32-125
        static $memo = make(  // f: 108-110
            1  // f: 32-125
        );  // f: 32-125
        $s = 'one  // f: 32-125
            } two';  // f: 32-125
        $q->for(  // f: 32-125
            1  // f: 32-125
        );  // f: 32-125
        Route::match(['get'], '/', function () {  // f: 32-125
            return 1;  // f: 32-125
        });  // f: 32-125
        $f = function ($x)  // f: 32-125
            use ($id) {  // f: 32-125
            return $x;  // f: 32-125
        };  // f: 32-125
        return $options['song']  // f: 32-125
            ?? null;  // f: 32-125
    }  // f: 32-125

    abstract protected function make(  // f: 127-129
        int $id  // f: 127-129
    ): Song;  // f: 127-129
}  // f: 20-130

interface Contract  // f: 132-135
{  // f: 132-135
    public function find(int $id): ?Song;  // f: 132-135
}  // f: 132-135

function helper(): void  // f: 137-140
{  // f: 137-140
    echo Song::class;  // f: 137-140
}  // f: 137-140

function &ref(  // f: 142-148
    array &$a  // f: 142-148
): array {  // f: 142-148
    return [  // f: 145-147
        $a,  // f: 142-148
    ];  // f: 142-148
}  // f: 142-148
?>
<p>It's done.</p>
<?php function tail()  // f: 151-154
{  // f: 151-154
    return 1;  // f: 151-154
}  // f: 151-154
<?php function glued($y)  // f: 155-162
{  // f: 155-162
    if ($y) {  // f: 157-160
        $z = $y# note } f: 155-162
        ;  // f: 155-162
    }  // f: 155-162
    return $z;  // f: 155-162
}  // f: 155-162
$o = new readonly class extends Base {  // f: none
    public function g()  // f: 164-167
    {  // f: 164-167
        return 1;  // f: 164-167
    }  // f: 164-167
};
$q = new #[Pure] readonly class implements Contract {  // f: none
    public $a;  // f: none
};
