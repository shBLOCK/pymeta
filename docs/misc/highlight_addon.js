hljs.registerLanguage("pymeta", (hljs) => {
    const CODE = {};
    CODE.contains = [
        // Groups
        {
            variants: [
                {begin: /\(/, end: /\)/},
                {begin: /\[/, end: /]/},
                {begin: /\{/, end: /}/},
            ],
            contains: [CODE]
        },

        // PyMeta chunks
        {
            begin: /\$/,
            end: hljs.MATCH_NOTHING_RE,
            beginScope: "pymeta.marker",
            scope: "pymeta.py",
            contains: [
                {
                    variants: [
                        {match: /\$/, scope: "pymeta.marker"},
                        {match: /;/},
                        {match: /:(?=\{)/}
                    ],
                    endsParent: true
                },

                // Python f-string and t-string prefix concat workaround (e.g. `f~"abc{i}"`)
                {
                    scope: 'string',
                    contains: [
                        hljs.BACKSLASH_ESCAPE,
                        {match: /\{\{|}}/},
                        {
                            scope: "subst",
                            begin: /\{/,
                            end: /}/,
                            subLanguage: "python"
                        }
                    ],
                    variants: [
                        {begin: /([fFtT][rR]|[rR][fFtT]|[fFtT])~'''/, end: /'''/},
                        {begin: /([fFtT][rR]|[rR][fFtT]|[fFtT])~"""/, end: /"""/},
                        {begin: /([fFtT][rR]|[rR][fFtT]|[fFtT])~'/, end: /'/},
                        {begin: /([fFtT][rR]|[rR][fFtT]|[fFtT])~"/, end: /"/}
                    ]
                },

                // other Python prefixes string literal workaround (e.g. `f~"a{i}"`)
                {
                    match: /[a-zA-Z]~(?=['"])/,
                    scope: "string"
                }
            ],
            subLanguage: "python",
        },

        // Rust
        {
            end: /(?=\$)/,
            subLanguage: "rust",
            contains: [
                // Rust comment
                hljs.C_LINE_COMMENT_MODE,
                hljs.COMMENT('/\\*', '\\*/', {contains: ['self']}),

                // Rust string
                hljs.inherit(hljs.QUOTE_STRING_MODE, {
                    begin: /b?"/,
                    illegal: null
                }),
                {
                    className: 'symbol',
                    // negative lookahead to avoid matching `'`
                    begin: /'[a-zA-Z_][a-zA-Z0-9_]*(?!')/
                },
                {
                    scope: 'string',
                    variants: [
                        {begin: /b?r(#*)"(.|\n)*?"\1(?!#)/},
                        {
                            begin: /b?'/,
                            end: /'/,
                            contains: [
                                {
                                    scope: "char.escape",
                                    match: /\\('|\w|x\w{2}|u\w{4}|U\w{8})/
                                }
                            ]
                        }
                    ]
                },

                // escape
                {
                    match: /<\$>/,
                    scope: "punctuation"
                },
            ]
        }
    ];

    return {
        name: "PyMeta",
        contains: [CODE]
    };
});