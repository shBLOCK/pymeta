from __future__ import annotations

import copy
import contextlib
import weakref
from abc import ABC, abstractmethod
from collections import deque
from collections.abc import Sequence

from typing import SupportsInt, SupportsFloat, final, Collection, Final, SupportsBytes, MutableSequence, overload, Self, \
    Any, Iterable

from ._ import native as _native
from ._.native import Span


__all__ = (
    "Span",
    "Token",
    "Tokens", "TokensView",
    "Group", "Punct", "Ident",
    "Literal", "IntLiteral", "FloatLiteral", "StrLiteral", "BytesLiteral",
    "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize",
    "f32", "f64",
    "lit",
    "emit",
)


class Token(ABC):
    __slots__ = ("span",)

    span: Span | None

    def __init__(self, span: Span | None = None):
        self.span = span

    @abstractmethod
    def __str__(self):
        ...

    @abstractmethod
    def __repr__(self):
        ...

    @abstractmethod
    def __eq__(self, other):
        ...

    def join(self, items: Iterable[CoerceToTokens]) -> Tokens:
        tokens = Tokens()
        first = True
        for item in items:
            if not first:
                tokens.append(self)
            first = False
            tokens.append(item)
        return tokens

    @abstractmethod
    def _append_to_tokenstream(self, stream: _native.TokenStream):
        ...


type CoerceToTokens = (
    Tokens | TokensView | Token
    | str
    | int | float | bool | (bytes | bytearray | memoryview[Any])
    | (tuple | list)
)


@final
class Tokens(MutableSequence[Token]):
    _CTX_STACK = deque()

    __slots__ = ("_tokens", "_views")

    @classmethod
    def _current_ctx(cls) -> Tokens:
        if not cls._CTX_STACK or cls._CTX_STACK[-1] is None:
            raise RuntimeError(f"No active {cls.__name__} context.")
        return cls._CTX_STACK[-1]

    @classmethod
    @contextlib.contextmanager
    def _none_ctx(cls):
        cls._CTX_STACK.append(None)
        try:
            yield
        finally:
            assert cls._CTX_STACK.pop() is None

    @staticmethod
    def _coerce(items: Iterable[CoerceToTokens], *, span: Span | None = None, out: list[Token] | None = None) -> list[
        Token]:
        _results = out if out is not None else []
        _group_stack: list[Group] = []

        def append(token: Token):
            if token.span is None:
                token.span = span
            if _group_stack:
                _group_stack[-1].tokens.append(token)
            else:
                _results.append(token)

        def push_group(delim: str):
            delim = Group.OPENING_TO_DELIMITER.get(delim)
            assert delim is not None
            _group_stack.append(Group(delim, span=span))

        def pop_group(delim: str):
            delim = Group.CLOSING_TO_DELIMITER.get(delim)
            assert delim is not None
            group = _group_stack.pop(-1)
            if group.delimiter != delim:
                raise ValueError(f"Group delimiter mismatch: opening={group.delimiter[0]}, closing={delim[1]}")
            append(group)

        def process_string(string: str):
            if string and all(c in Punct.CHARS for c in string):
                for c in string[:-1]:
                    append(Punct(c, Punct.JOINT))
                append(Punct(string[-1], Punct.ALONE))
            elif Ident._is_valid_ident(string):
                append(Ident(string))
            else:
                raise ValueError(
                    f"\"{string}\" is not a valid Rust identifier nor Rust punctuations. "
                    "If this is intended to be a string literal, use `lit(<string>)`. "
                    "Or use `Tokens.parse()` to parse the string as Rust code."
                )

        def process_one(item: CoerceToTokens):
            match item:
                case Tokens() | TokensView():
                    for token in item:
                        append(token)
                case Token():
                    append(item)
                case str(string):
                    process_string(string)
                case int(value):
                    append(IntLiteral(value))
                case float(value):
                    append(FloatLiteral(value))
                case bool(value):
                    append(Ident("true" if value else "false"))
                case bytes(bts) | bytearray(bts) | (memoryview() as bts):
                    append(BytesLiteral(bts))
                case tuple(tup):
                    append(Group(Group.PARENTHESIS, Tokens(items=tup)))
                case list(lst):
                    append(Group(Group.BRACKET, Tokens(items=lst)))
                case _:
                    raise TypeError(f"Item {item!r} or type {type(item)} can't be coerced into tokens.")

        for item in items:
            process_one(item)

        return _results

    _tokens: list[Token]
    _views: weakref.WeakSet[TokensView] | None

    def __init__(
        self,
        *args: CoerceToTokens,
        items: Iterable[CoerceToTokens] | None = None,
        tokens: Iterable[Token] | None = None,
        span: Span | None = None
    ):
        match (args, items, tokens):
            case (_, None, None):
                self._tokens = Tokens._coerce(args, span=span)
            case ((), items, None) if items is not None:
                self._tokens = Tokens._coerce(items, span=span)
            case ((), None, tokens) if tokens is not None:
                self._tokens = list(tokens)
                for token in self._tokens:
                    if not isinstance(token, Token):
                        raise TypeError(f"Not a Token: {token!r}")
                    if token.span is None:
                        token.span = span
            case _:
                raise ValueError("Multiple arg collections provided")

        self._views = None

    def __enter__(self) -> Self:
        Tokens._CTX_STACK.append(self)
        return self

    def __exit__(self, *_):
        top = Tokens._CTX_STACK.pop()
        assert top is self

    def __str__(self):
        return " ".join(map(str, self._tokens))

    def __repr__(self):
        return f"{self.__class__.__name__}({", ".join(map(repr, self._tokens))})"

    def __len__(self):
        return self._tokens.__len__()

    @overload
    def __getitem__(self, index: int) -> Token:
        ...

    @overload
    def __getitem__(self, index: slice) -> TokensView:
        ...

    def __getitem__(self, index):
        if isinstance(index, int):
            return self._tokens.__getitem__(index)
        elif isinstance(index, slice):
            if index.step is not None:
                raise NotImplementedError("stepped slicing is not supported")
            return TokensView(self, index.start, index.stop)
        else:
            raise TypeError(index)

    def _track_view(self, view: TokensView):
        if self._views is None:
            self._views = weakref.WeakSet()
        self._views.add(view)

    # region mutable operations
    def _check_mutation(self):
        if self._views:
            raise RuntimeError(f"Can not mutate a {self.__class__.__name__} while views to it are active")

    def __setitem__(self, index, value):
        self._check_mutation()
        self._tokens.__setitem__(index, value)

    def __delitem__(self, index):
        self._check_mutation()
        self._tokens.__delitem__(index)

    def insert(self, index, value):
        self._check_mutation()
        return self._tokens.insert(index, value)

    def append(self, *args: CoerceToTokens):
        self._check_mutation()
        Tokens._coerce(args, out=self._tokens)

    def extend(self, items: Iterable[CoerceToTokens]):
        self._check_mutation()
        Tokens._coerce(items, out=self._tokens)

    def reverse(self):
        raise NotImplementedError

    # endregion

    def __reversed__(self):
        raise NotImplementedError

    def _to_tokenstream(self) -> _native.TokenStream:
        stream = _native.TokenStream()
        for token in self:
            token._append_to_tokenstream(stream)
        return stream


@final
class TokensView(Sequence[Token]):
    __slots__ = ("_referent", "_pos", "_end", "__weakref__")

    def __init__(self, referent: Tokens, pos: int | None, end: int | None):
        self._referent = referent
        referent._track_view(self)
        pos = pos or 0
        end = end or len(referent)
        if pos < 0:
            pos = len(referent) + pos
        if end < 0:
            end = len(referent) + end
        self._pos = pos
        self._end = end
        self._check_slice()

    def _check_slice(self):
        if self._pos > self._end:
            raise IndexError(f"pos({self._pos}) > end({self._end})")

    @property
    def referent(self):
        return self._referent

    @property
    def pos(self) -> int:
        return self._pos

    @pos.setter
    def pos(self, index: int):
        index = index or 0
        if index < 0:
            index = len(self._referent) + index
        self._pos = index
        self._check_slice()

    @property
    def end(self) -> int:
        return self._end

    @end.setter
    def end(self, index: int):
        index = index or 0
        if index < 0:
            index = len(self._referent) + index
        self._end = index
        self._check_slice()

    def _map_index(self, index: int) -> int:
        if index >= 0:
            if index >= len(self):
                raise IndexError()
            return self.pos + index
        else:
            if -index > len(self):
                raise IndexError()
            return self.end + index

    @overload
    def __getitem__(self, index: int, /) -> Token:
        ...

    @overload
    def __getitem__(self, index: slice, /) -> Self:
        ...

    def __getitem__(self, index, /):
        if isinstance(index, int):
            return self._referent[self._map_index(index)]
        elif isinstance(index, slice):
            if index.step is not None:
                raise NotImplementedError("stepped slicing is not supported")
            return TokensView(self._referent, self._map_index(index.start), self._map_index(index.stop))
        else:
            raise TypeError(index)

    def __len__(self) -> int:
        return self.end - self.pos


@final
class Group(Token):
    PARENTHESIS: Final[str] = "()"
    BRACKET: Final[str] = "[]"
    BRACE: Final[str] = "{}"
    NONE: Final[str] = ""
    DELIMITERS: Final[Collection[str]] = (PARENTHESIS, BRACKET, BRACE, NONE)
    OPENING_TO_DELIMITER: Final[dict[str, str]] = {"(": "()", "[": "[]", "{": "{}"}
    CLOSING_TO_DELIMITER: Final[dict[str, str]] = {")": "()", "]": "[]", "}": "{}"}

    __slots__ = ("delimiter", "tokens")
    __match_args__ = ("delimiter", "tokens")

    delimiter: str
    tokens: Tokens

    def __init__(self, delimiter: str, tokens: Tokens | None = None, span: Span | None = None):
        super().__init__(span)
        if delimiter not in Group.DELIMITERS:
            raise ValueError(f"invalid group delimiter: \"{delimiter}\"")
        self.delimiter = delimiter
        self.tokens = tokens if tokens is not None else Tokens()

    @property
    def opening(self) -> str:
        return self.delimiter[0] if self.delimiter != "" else ""

    @property
    def closing(self) -> str:
        return self.delimiter[1] if self.delimiter != "" else ""

    def __str__(self):
        delim = self.delimiter if self.delimiter != "" else "∅∅"
        return f"{delim[0]} {self.tokens} {delim[1]}"

    def __repr__(self):
        return f"{self.__class__.__name__}(\"{self.delimiter}\", {self.tokens!r}, {self.span!r})"

    def __eq__(self, other):
        """Compare if delimiter types are equal, group contents (tokens) are ignored."""
        if not isinstance(other, Group):
            return False
        return self.delimiter == other.delimiter

    def __enter__(self):
        return self.tokens.__enter__()

    def __exit__(self, *_):
        self.tokens.__exit__()

    def _append_to_tokenstream(self, stream: _native.TokenStream):
        stream.append_group(self.delimiter, self.tokens._to_tokenstream(), self.span)


@final
class Ident(Token):
    __slots__ = ("string",)
    __match_args__ = ("string",)

    string: str

    @staticmethod
    def _is_valid_ident(string: str) -> bool:
        if string.startswith("r#"):
            string = string[2:]
        if not string:
            return False
        if not _native.is_ident_start(string[0]):
            return False
        return all(_native.is_ident_continue(c) for c in string[1:])

    def __init__(self, string: str, span: Span | None = None):
        super().__init__(span)
        self.string = string

    def __str__(self):
        return self.string

    def __repr__(self):
        return f"{self.__class__.__name__}({self.string!r}, {self.span!r})"

    def __eq__(self, other):
        if not isinstance(other, Ident):
            return False
        return self.string == other.string

    def _append_to_tokenstream(self, stream: _native.TokenStream):
        stream.append_ident(self.string, self.span)


@final
class Punct(Token):
    CHARS: Final[Collection[str]] = tuple("=<>!~+-*/%^&|@.,;:#$?'")
    ALONE: Final[str] = "alone"
    JOINT: Final[str] = "joint"

    __slots__ = ("char", "spacing")
    __match_args__ = ("char", "spacing")

    char: str
    spacing: str

    def __init__(self, char: str, spacing: str = ALONE, span: Span | None = None):
        super().__init__(span)
        if len(char) != 1:
            raise ValueError(f"Punct only accept a single char, got \"{char}\"")
        if char not in Punct.CHARS:
            raise ValueError(f"Invalid punctuation char '{char}'")
        self.char = char
        if spacing not in (Punct.ALONE, Punct.JOINT):
            raise ValueError(f"Invalid punctuation spacing type: \"{spacing}\"")
        self.spacing = spacing

    def __str__(self):
        return self.char

    def __repr__(self):
        return f"{self.__class__.__name__}({self.char!r}, {self.span!r})"

    def __eq__(self, other):
        if not isinstance(other, Punct):
            return False
        return self.char == other.char and self.spacing == other.spacing

    def _append_to_tokenstream(self, stream: _native.TokenStream):
        stream.append_punct(self.char, self.spacing, self.span)


class Literal[T](Token, ABC):
    __slots__ = ("value", "repr")
    __match_args__ = ("value",)

    value: T


@final
class IntLiteral(Literal[int]):
    @final
    class Type:
        __slots__ = ("suffix", "signed")

        _TYPES = {}

        @overload
        def __new__(cls, suffix: str) -> Self: ...

        @overload
        def __new__(cls, suffix: str, signed: bool) -> Self: ...

        def __new__(cls, suffix: str, signed: bool | None = None) -> Self:
            obj = cls._TYPES.get(suffix)
            if obj is None:
                obj = super().__new__(cls)
                obj.suffix = suffix
                obj.signed = signed
                cls._TYPES[suffix] = obj
            return obj

        def __call__(self, value: SupportsInt, span: Span | None = None):
            return IntLiteral(value, self, span)

        def __repr__(self):
            return self.suffix

    __slots__ = ("repr", "type")

    repr: str | None
    type: Type | None

    @classmethod
    def _new(cls, repr: str, value: int, type: Type | None, span: Span) -> Self:
        """For use from generated code only."""
        obj = cls.__new__(cls)
        obj.repr = repr
        obj.value = value
        obj.type = type
        obj.span = span
        return obj

    def __init__(self, value: SupportsInt, type: Type | str | None = None, span: Span | None = None):
        super().__init__(span)
        self.value = int(value)
        if isinstance(type, str):
            type = IntLiteral.Type(type)
        self.type = type

    def __str__(self):
        return f"{self.repr or self.value}{self.type or ""}"

    def __repr__(self):
        return f"{self.__class__.__name__}({self.repr or self.value}, {self.type}, {self.span!r})"

    def __eq__(self, other):
        if not isinstance(other, IntLiteral):
            return False
        return self.value == other.value and self.type == other.type


    def _append_to_tokenstream(self, stream: _native.TokenStream):
        stream.append_int_literal(self.value, str(self.type) if self.type else None, self.span)


# @formatter:off
u8 = IntLiteral.Type("u8", False)
u16 = IntLiteral.Type("u16", False)
u32 = IntLiteral.Type("u32", False)
u64 = IntLiteral.Type("u64", False)
u128 = IntLiteral.Type("u128", False)
usize = IntLiteral.Type("usize", False)
i8 = IntLiteral.Type("i8", True)
i16 = IntLiteral.Type("i16", True)
i32 = IntLiteral.Type("i32", True)
i64 = IntLiteral.Type("i64", True)
i128 = IntLiteral.Type("i128", True)
isize = IntLiteral.Type("isize", True)
# @formatter:on


class FloatLiteral(Literal[float]):
    @final
    class Type:
        __slots__ = ("suffix", "bits")

        _TYPES = {}

        @overload
        def __new__(cls, suffix: str) -> Self: ...

        @overload
        def __new__(cls, suffix: str, bits: int) -> Self: ...

        def __new__(cls, suffix: str, bits: int | None = None) -> Self:
            obj = cls._TYPES.get(suffix)
            if obj is None:
                obj = super().__new__(cls)
                obj.suffix = suffix
                obj.bits = bits
                cls._TYPES[suffix] = obj
            return obj

        def __call__(self, value: SupportsFloat, span: Span | None = None):
            return FloatLiteral(value, self, span)

        def __repr__(self):
            return self.suffix

    __slots__ = ("repr", "type")

    repr: str | None
    type: Type | None

    @classmethod
    def _new(cls, repr: str, value: float, type: Type | None, span: Span) -> Self:
        """For use from generated code only."""
        obj = cls.__new__(cls)
        obj.repr = repr
        obj.value = value
        obj.type = type
        obj.span = span
        return obj

    def __init__(self, value: SupportsFloat, type: Type | str | None = None, span: Span | None = None):
        super().__init__(span)
        self.value = float(value)
        if isinstance(type, str):
            type = FloatLiteral.Type(type)
        self.type = type

    def __str__(self):
        return f"{self.repr or self.value}{self.type or ""}"

    def __repr__(self):
        return f"{self.__class__.__name__}({self.repr or self.value}, {self.type}, {self.span!r})"

    def __eq__(self, other):
        if not isinstance(other, FloatLiteral):
            return False
        return self.value == other.value and self.type == other.type

    def _append_to_tokenstream(self, stream: _native.TokenStream):
        stream.append_float_literal(self.value, str(self.type) if self.type else None, self.span)


# @formatter:off
f32 = FloatLiteral.Type("f32", 32)
f64 = FloatLiteral.Type("f64", 64)
# @formatter:on


class StrLiteral(Literal[str]):
    CHR: Final[str] = "chr"
    STR: Final[str] = "str"

    __slots__ = ("type",)

    type: str

    def __init__(self, value: str, type: str = STR, span: Span | None = None):
        super().__init__(span)
        if type not in (StrLiteral.STR, StrLiteral.CHR):
            raise ValueError(f"invalid {self.__class__.__name__} type \"{type}\"")
        if type == StrLiteral.CHR and len(value) != 1:
            raise ValueError(f"\"{value}\" is not a single character")
        self.value = value
        self.type = type

    def __str__(self):
        return f'"{self.repr}"' if self.repr is not None else repr(self.value)  # TODO: rust format

    def __repr__(self):
        return f"{self.__class__.__name__}({self.__str__()}, \"{self.type}\", {self.span!r})"

    def __eq__(self, other):
        if not isinstance(other, StrLiteral):
            return False
        return self.type == other.type and self.value == other.value

    def _append_to_tokenstream(self, stream: _native.TokenStream):
        stream.append_str_literal(self.type, self.value, self.span)


class BytesLiteral(Literal[bytes]):
    BYTE: Final[str] = "byte"
    BYTES: Final[str] = "bytes"
    CSTR: Final[str] = "cstr"

    __slots__ = ("type",)

    type: str

    def __init__(self, value: SupportsBytes, type: str = BYTES, span: Span | None = None):
        super().__init__(span)
        value = bytes(value)
        if type not in (BytesLiteral.BYTE, BytesLiteral.BYTES, BytesLiteral.CSTR):
            raise ValueError(f"invalid {self.__class__.__name__} type \"{type}\"")
        if type == BytesLiteral.BYTE and len(value) != 1:
            raise ValueError(f"\"{value}\" is not a single byte")
        self.value = value
        self.type = type

    def __str__(self):
        return f'"{self.repr}"' if self.repr is not None else repr(self.value)  # TODO: rust format

    def __repr__(self):
        return f"{self.__class__.__name__}({self.__str__()}, \"{self.type}\", {self.span!r})"

    def __eq__(self, other):
        if not isinstance(other, BytesLiteral):
            return False
        return self.type == other.type and self.value == other.value

    def _append_to_tokenstream(self, stream: _native.TokenStream):
        stream.append_bytes_literal(self.type, self.value, self.span)


# @formatter:off
@overload
def lit(value: str, /, *, span: Span | None = None) -> StrLiteral: ...
@overload
def lit(value: bytes | bytearray | memoryview, /, *, span: Span | None = None) -> BytesLiteral: ...
@overload
def lit(*, chr: Any, span: Span | None = None) -> StrLiteral: ...
@overload
def lit(*, str: Any, span: Span | None = None) -> StrLiteral: ...
@overload
def lit(*, cstr: Any, span: Span | None = None) -> StrLiteral: ...
@overload
def lit(*, byte: SupportsBytes | int, span: Span | None = None) -> BytesLiteral: ...
@overload
def lit(*, bytes: SupportsBytes, span: Span | None = None) -> BytesLiteral: ...
@overload
def lit(value: int, /, *, span: Span | None = None) -> IntLiteral: ...
@overload
def lit(value: float, /, *, span: Span | None = None) -> FloatLiteral: ...
@overload
def lit(value: bool, /, *, span: Span | None = None) -> Ident: ...
@overload
def lit[T: Literal | Ident](value: T, /, *, span: Span | None = None) -> T: ...
# @formatter:on

def lit(
    value=None,
    /, *,
    span=None,
    **kwargs,
):
    if len(kwargs) == 0:
        kwarg = ()
    elif len(kwargs) == 1:
        kwarg = next(iter(kwargs.items()))
    else:
        raise ValueError("Received multiple values")

    match (value, kwarg):
        case (str(string), ()) | (None, ("str", string)):
            return StrLiteral(str(string), StrLiteral.STR, span)
        case (bytes(bts) | bytearray(bts) | (memoryview() as bts), ()) | (None, ("bytes", bts)):
            return BytesLiteral(bytes(bts), BytesLiteral.BYTES, span)
        case (None, ("chr", char)):
            return StrLiteral(str(char), StrLiteral.CHR, span)
        case (None, ("cstr", str(cstr))):
            return BytesLiteral(cstr.encode(), BytesLiteral.CSTR, span)
        case (None, ("byte", SupportsBytes() as byte)):
            return BytesLiteral(byte, BytesLiteral.BYTE, span)
        case (None, ("byte", int(byte))):
            return BytesLiteral(byte.to_bytes(signed=True))
        case (None, ("byte", invalid_byte)):
            raise TypeError(f"{invalid_byte!r} of type {type(invalid_byte)} can't be converted into a byte literal")
        case (int(int_value), ()):
            return IntLiteral(int_value, span=span)
        case (float(float_value), ()):
            return FloatLiteral(float_value, span=span)
        case (bool(boolean), ()):
            return Ident("true" if boolean else "false", span)
        case Literal() as literal:
            literal = copy.copy(literal)
            if span is not None:
                literal.span = span
            return literal
        case Ident() as ident:
            if ident.string == "false":
                boolean = False
            elif ident.string == "true":
                boolean = True
            else:
                raise ValueError(f"{ident!r} is not a boolean literal identifier")
            return lit(boolean, span=span)
        case (None, ()):
            raise ValueError("Expected one literal value, got none")
        case (value, (_, _)) if value is not None:
            raise ValueError("Received multiple values")
        case (value, ()):
            raise ValueError(f"Invalid value: {value}")
        case (None, (key, value)):
            raise ValueError(f"Invalid type and/or value: {key=}, {value=}")
    assert not "reachable"


# noinspection PyProtectedMember
def emit(*args: CoerceToTokens, span: Span | None = None) -> None | Group:
    """Take a list of items, coerce them into ``Tokens``, then add them to the current ``Tokens`` context.

    :return: The last ``Token`` coerced from args if that token is a ``Group``, ``None`` elsewise.
        The intended use case for this is to be used together with a ``with`` statement
        to enter the ``Group`` at the end of the coercion result.
    """
    tokens = Tokens(*args, span=span)
    Tokens._current_ctx().extend(tokens)
    if len(tokens) > 0 and isinstance(tokens[-1], Group):
        return tokens[-1]
    return None
