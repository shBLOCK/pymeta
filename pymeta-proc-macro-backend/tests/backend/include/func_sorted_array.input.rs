sorted_array! {
    key = key or (lambda x: x);
    items = sorted(items_dict.items(), key=lambda kv: key(kv[0]));
    // The semi-quoting syntax (`{{...}}`) is very useful here.
    return {{
        const $name$: [$typ$; $len(items)$] = [
            $for k,v in items:{
                ($k$, $v$),
            }
        ];
    }};
}