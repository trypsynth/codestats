{{-- expect: lang="Blade" total=10 code=7 comment=2 blank=1 shebang=0 --}}
@extends('layouts.app')

@section('content')
    {{-- Show each user. --}}
    @foreach ($users as $user)
        <p>{{ $user->name }}</p>
    @endforeach
    @include('partials.footer')
@endsection
