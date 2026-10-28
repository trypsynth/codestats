// expect: lang="PHP" total=8 code=6 comment=1 blank=1 shebang=0
<?php

return [
    'paths' => ['api/*', 'sanctum/csrf-cookie'],
    'allowed_origins' => ['*'],
    "note" => "it's fine",
];
