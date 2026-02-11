from abc import ABC, abstractmethod
from collections import deque
from string.templatelib import Template
from typing import SupportsInt, SupportsFloat, final, Collection, Final, SupportsBytes, MutableSequence, overload, Self, \
    Any, Iterable

import _pymeta
from _pymeta import Span


__all__ = (
    "Span",
    "Token",
    "Tokens",
    "Group", "Punct", "Ident",
    "Literal", "IntLiteral", "FloatLiteral", "StrLiteral", "BytesLiteral",
    "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize",
    "f32", "f64",
    "lit",
    "rust",
)


class Token(ABC):
    __slots__ = ("span",)

    span: Span

    def __init__(self, span: Span | None = None):
        self.span = span or Span.call_site()

    @abstractmethod
    def __str__(self): ...

    @abstractmethod
    def __repr__(self): ...

    @abstractmethod
    def _append_to_tokenstream(self, stream: _pymeta.TokenStream): ...


type CoerceToTokens = Token | Template | str | int | float | (bytes | bytearray | memoryview[Any]) | (tuple | list)

@final
class Tokens(MutableSequence[Token]):
    _CTX_STACK = deque()

    __slots__ = ("_tokens",)

    @classmethod
    def _current_ctx(cls) -> Tokens:
        return cls._CTX_STACK[-1]

    @staticmethod
    def _coerce(items: Iterable[CoerceToTokens]) -> list[Token]:
        _results = []
        _group_stack: list[Group] = []

        def emit(token: Token):
            if _group_stack:
                _group_stack[-1].tokens.append(token)
            else:
                _results.append(token)

        def push_group(delim: str):
            delim = Group.OPENING_TO_DELIMITER.get(delim)
            assert delim is not None
            _group_stack.append(Group(delim))

        def pop_group(delim: str):
            delim = Group.CLOSING_TO_DELIMITER.get(delim)
            assert delim is not None
            group = _group_stack.pop(-1)
            if group.delimiter != delim:
                raise ValueError(f"Group delimiter mismatch: opening={group.delimiter[0]}, closing={delim[1]}")
            emit(group)

        def parse(string: str):
            index = 0

            def parse_punct() -> bool:
                nonlocal index

                if string[index] not in Punct.CHARS:
                    return False

                start = index
                index += 1
                while index < len(string) and string[index] in Punct.CHARS:
                    index += 1

                # lifetime
                spacing = Punct.JOINT \
                    if string[index - 1] == "'" and (index < len(string) and _pymeta.is_ident_start(string[index])) \
                    else Punct.ALONE

                emit(Punct(string[start:index], spacing))
                return True

            def parse_number() -> bool:
                import re
                nonlocal index

                def is_dec_digit(char: str) -> bool:
                    return ord('0') <= ord(char) <= ord('9')

                def is_digit(char: str) -> bool:
                    char = ord(char)
                    return (ord('0') <= char <= ord('9')
                            or ord('a') <= char <= ord('f')
                            or ord('A') <= char <= ord('F'))

                start = index

                # if string[index] == "-":
                #     if not is_dec_digit(string[index + 1]):
                #         return False
                #     index += 1
                # else:
                #     if not is_dec_digit(string[index]):
                #         return False

                if not is_dec_digit(string[index]):
                    return False

                index += 1
                found_dot = False
                while index < len(string):
                    char = string[index]
                    if is_digit(char) or char in ('_', 'b', 'o', 'x', 'e', 'E', 'u', 'i', 'f'):
                        continue
                    if char == '.':
                        if found_dot:
                            break
                        if (
                            (index + 1) < len(string)
                            and (string[index + 1] == '.' or _pymeta.is_ident_start(string[index + 1]))
                        ):
                            break
                        found_dot = True
                        continue
                    break

                segment = string[start:index]
                # int
                if match := re.fullmatch(
                    r"(?P<num>(?:0[bo])?[\d_]+|0x[\da-fA-F_]+)(?P<suffix>[ui]\d+)?",
                    segment, flags=re.ASCII
                ):
                    emit(IntLiteral(int(match.group("num")), match.group("suffix") or None))
                # float
                elif match := re.fullmatch(
                    r"(?P<num>\d+\.\d*(?:[eE][+-]?\d+)?)(?P<suffix>f\d+)?",
                    segment, flags=re.ASCII
                ):
                    emit(FloatLiteral(float(match.group("num")), match.group("suffix") or None))
                else:
                    raise ValueError(f"Invalid number literal: {segment!r}")

                return True

            def parse_str_literal() -> bool:
                nonlocal index

                if string[index] in ('b', 'c'):
                    prefix = string[index]
                    if not ((index + 1) < len(string) and string[index + 1] in ('"', "'")):
                        return False
                    quote = string[index + 1]
                    index += 2
                else:
                    prefix = None
                    if not string[index] in ('"', "'"):
                        return False
                    quote = string[index]
                    index += 1

                content = []
                while index < len(string):
                    if string[index] == quote:
                        index += 1
                        break
                    match tuple(string[index:index + 3]):
                        case ('\\', '"', *_):
                            content.append('"')
                            index += 2
                        case ('\\', "'", *_):
                            content.append("'")
                            index += 2
                        case ('\\', '\\', '"'):
                            content.append(r'\"')
                            index += 3
                        case ('\\', '\\', "'"):
                            content.append(r"\'")
                            index += 3
                        case (char, *_):
                            content.append(char)
                            index += 1
                else:
                    raise ValueError("Incomplete string literal")

                content = "".join(content)
                is_char = quote == "'"
                match prefix:
                    case None:
                        emit(StrLiteral(content, StrLiteral.CHR if is_char else StrLiteral.STR))
                    case 'b':
                        emit(BytesLiteral(content.encode(), BytesLiteral.BYTE if is_char else BytesLiteral.BYTES))
                    case 'c':
                        emit(StrLiteral(content, StrLiteral.CSTR))
                    case _:
                        assert not "reachable"

                return True

            while index < len(string):
                char = string[index]
                if char.isspace():
                    index += 1
                elif char in "([{":
                    push_group(char)
                    index += 1
                elif char in ")]}":
                    pop_group(char)
                    index += 1
                elif parse_punct():
                    pass
                elif _pymeta.is_ident_start(char):
                    start = index
                    index += 1
                    while index < len(string) and _pymeta.is_ident_continue(string[index]):
                        index += 1
                    emit(Ident(string[start:index]))
                elif parse_number():
                    pass
                elif parse_str_literal():
                    pass
                else:
                    raise ValueError(f"Invalid syntax near {string[max(index - 4, 0):index + 5]!r}")

        def process_one(item: CoerceToTokens):
            match item:
                case Token():
                    emit(item)
                case int(value):
                    emit(IntLiteral(value))
                case float(value):
                    emit(FloatLiteral(value))
                case bytes(bts) | bytearray(bts) | memoryview(bts):
                    emit(BytesLiteral(bts))
                case tuple(tup):
                    emit(Group(Group.PARENTHESIS, Tokens(items=tup)))
                case list(lst):
                    emit(Group(Group.BRACKET, Tokens(items=lst)))
                case Template() as template:
                    for part in template:
                        if isinstance(part, str):
                            parse(part)
                        else:
                            process_one(part)
                case str(string):
                    parse(string)
                case _:
                    raise TypeError(f"Item {item!r} or type {type(item)} can't be coerced into tokens.")

        for item in items:
            process_one(item)

        return _results

    def __init__(
        self,
        *args: CoerceToTokens,
        items: Iterable[CoerceToTokens] | None = None,
        tokens: Iterable[Token] | None = None
    ):
        match (args, items, tokens):
            case (_, None, None):
                self._tokens = Tokens._coerce(args)
            case ((), items, None) if items is not None:
                self._tokens = Tokens._coerce(items)
            case ((), None, tokens) if tokens is not None:
                self._tokens = list(tokens)
                for token in self._tokens:
                    if not isinstance(token, Token):
                        raise TypeError(f"Not a Token: {token!r}")
            case _:
                raise ValueError("Multiple arg collections provided")

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

    def insert(self, index, value):
        return self._tokens.insert(index, value)

    @overload
    def __getitem__(self, index: int) -> Token:
        ...

    @overload
    def __getitem__(self, index: slice) -> Tokens:
        ...

    def __getitem__(self, index):
        result = self._tokens.__getitem__(index)
        if isinstance(result, list):
            result = Tokens(*result)
        return result

    def __setitem__(self, index, value):
        self._tokens.__setitem__(index, value)

    def __delitem__(self, index):
        self._tokens.__delitem__(index)

    def __len__(self):
        return self._tokens.__len__()

    def append(self, value: Token):
        self._tokens.append(value)

    def extend(self, values):
        self._tokens.extend(values)

    def reverse(self):
        raise NotImplementedError

    def __reversed__(self):
        raise NotImplementedError

    def _to_tokenstream(self) -> _pymeta.TokenStream:
        stream = _pymeta.TokenStream()
        for token in self:
            token._append_to_tokenstream(stream)
        return stream


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

    def __enter__(self):
        return self.tokens.__enter__()

    def __exit__(self, *_):
        self.tokens.__exit__()

    def _append_to_tokenstream(self, stream: _pymeta.TokenStream):
        stream.append_group(self.delimiter, self.tokens._to_tokenstream(), self.span)


@final
class Ident(Token):
    __slots__ = ("string",)
    __match_args__ = ("string",)

    string: str

    def __init__(self, string: str, span: Span | None = None):
        super().__init__(span)
        self.string = string

    def __str__(self):
        return self.string

    def __repr__(self):
        return f"{self.__class__.__name__}({self.string!r}, {self.span!r})"

    def _append_to_tokenstream(self, stream: _pymeta.TokenStream):
        stream.append_ident(self.string, self.span)


@final
class Punct(Token):
    CHARS: Final[Collection[str]] = tuple("=<>!~+-*/%^&|@.,;:#$?'")
    ALONE: Final[str] = "alone"
    JOINT: Final[str] = "joint"

    __slots__ = ("chars", "spacing")
    __match_args__ = ("chars", "spacing")

    chars: str
    spacing: str

    def __init__(self, chars: str, spacing: str = ALONE, span: Span | None = None):
        super().__init__(span)
        for c in chars:
            if c not in Punct.CHARS:
                raise ValueError(f"Invalid punctuation char '{c}'")
        self.chars = chars
        if spacing not in (Punct.ALONE, Punct.JOINT):
            raise ValueError(f"Invalid punctuation spacing type: \"{spacing}\"")
        self.spacing = spacing

    def __str__(self):
        return self.chars

    def __repr__(self):
        return f"{self.__class__.__name__}({self.chars!r}, {self.span!r})"

    def _append_to_tokenstream(self, stream: _pymeta.TokenStream):
        if len(self.chars) == 0:
            raise ValueError("Punct token is empty")
        for char in self.chars[:-1]:
            stream.append_punct(char, Punct.JOINT, self.span)
        stream.append_punct(self.chars[-1], self.spacing, self.span)


class Literal[T](Token, ABC):
    __slots__ = ("value",)
    __match_args__ = ("value",)

    value: T


@final
class IntLiteral(Literal[int]):
    SUFFIXES: Final[Collection[str]] = tuple(a + b for a in "ui" for b in ("8", "16", "32", "64", "128", "size"))

    __slots__ = ("suffix",)

    suffix: str | None

    def __init__(self, value: SupportsInt, suffix: str | None = None, span: Span | None = None):
        super().__init__(span)
        self.value = int(value)
        if suffix is not None and suffix not in IntLiteral.SUFFIXES:
            raise ValueError(f"invalid {self.__class__.__name__} suffix \"{suffix}\"")
        self.suffix = suffix

    def __str__(self):
        return f"{self.value}{self.suffix or ""}"

    def __repr__(self):
        return f"{self.__class__.__name__}({self.value!r}, \"{self.suffix}\", {self.span!r})"

    def _append_to_tokenstream(self, stream: _pymeta.TokenStream):
        stream.append_int_literal(self.value, self.suffix, self.span)

# @formatter:off
def u8(value: SupportsInt, span: Span | None = None) -> IntLiteral:
    return IntLiteral(value, "u8", span)
def u16(value: SupportsInt, span: Span | None = None) -> IntLiteral:
    return IntLiteral(value, "u16", span)
def u32(value: SupportsInt, span: Span | None = None) -> IntLiteral:
    return IntLiteral(value, "u32", span)
def u64(value: SupportsInt, span: Span | None = None) -> IntLiteral:
    return IntLiteral(value, "u64", span)
def u128(value: SupportsInt, span: Span | None = None) -> IntLiteral:
    return IntLiteral(value, "u128", span)
def usize(value: SupportsInt, span: Span | None = None) -> IntLiteral:
    return IntLiteral(value, "usize", span)
def i8(value: SupportsInt, span: Span | None = None) -> IntLiteral:
    return IntLiteral(value, "i8", span)
def i16(value: SupportsInt, span: Span | None = None) -> IntLiteral:
    return IntLiteral(value, "i16", span)
def i32(value: SupportsInt, span: Span | None = None) -> IntLiteral:
    return IntLiteral(value, "i32", span)
def i64(value: SupportsInt, span: Span | None = None) -> IntLiteral:
    return IntLiteral(value, "i64", span)
def i128(value: SupportsInt, span: Span | None = None) -> IntLiteral:
    return IntLiteral(value, "i128", span)
def isize(value: SupportsInt, span: Span | None = None) -> IntLiteral:
    return IntLiteral(value, "isize", span)
# @formatter:on


class FloatLiteral(Literal[float]):
    SUFFIXES: Final[Collection[str]] = ("f32", "f64")

    __slots__ = ("suffix",)

    suffix: str | None

    def __init__(self, value: SupportsFloat, suffix: str | None = None, span: Span | None = None):
        super().__init__(span)
        self.value = float(value)
        if suffix is not None and suffix not in FloatLiteral.SUFFIXES:
            raise ValueError(f"invalid {self.__class__.__name__} suffix \"{suffix}\"")
        self.suffix = suffix

    def __str__(self):
        return f"{self.value}{self.suffix or ""}"

    def __repr__(self):
        return f"{self.__class__.__name__}({self.value!r}, \"{self.suffix}\", {self.span!r})"

    def _append_to_tokenstream(self, stream: _pymeta.TokenStream):
        stream.append_float_literal(self.value, self.suffix, self.span)

def f32(value: SupportsFloat, span: Span | None = None) -> FloatLiteral:
    return FloatLiteral(value, "f32", span)

def f64(value: SupportsFloat, span: Span | None = None) -> FloatLiteral:
    return FloatLiteral(value, "f64", span)


class StrLiteral(Literal[str]):
    CHR: Final[str] = "chr"
    STR: Final[str] = "str"
    CSTR: Final[str] = "cstr"

    __slots__ = ("type",)

    type: str

    def __init__(self, value: str, type: str = STR, span: Span | None = None):
        super().__init__(span)
        if type not in (StrLiteral.STR, StrLiteral.CHR, StrLiteral.CSTR):
            raise ValueError(f"invalid {self.__class__.__name__} type \"{type}\"")
        if type == StrLiteral.CHR and len(value) != 1:
            raise ValueError(f"\"{value}\" is not a single character")
        self.value = value
        self.type = type

    def __str__(self):
        return repr(self.value)  # TODO: rust format

    def __repr__(self):
        return f"{self.__class__.__name__}({self.value!r}, \"{self.type}\", {self.span!r})"

    def _append_to_tokenstream(self, stream: _pymeta.TokenStream):
        stream.append_str_literal(self.type, self.value, self.span)


class BytesLiteral(Literal[bytes]):
    BYTE: Final[str] = "byte"
    BYTES: Final[str] = "bytes"

    __slots__ = ("type",)

    type: str

    def __init__(self, value: SupportsBytes, type: str = BYTES, span: Span | None = None):
        super().__init__(span)
        value = bytes(value)
        if type not in (BytesLiteral.BYTE, BytesLiteral.BYTES):
            raise ValueError(f"invalid {self.__class__.__name__} type \"{type}\"")
        if type == BytesLiteral.BYTE and len(value) != 1:
            raise ValueError(f"\"{value}\" is not a single byte")
        self.value = value
        self.type = type

    def __str__(self):
        return repr(self.value)  # TODO: rust format

    def __repr__(self):
        return f"{self.__class__.__name__}({self.value!r}, \"{self.type}\", {self.span!r})"

    def _append_to_tokenstream(self, stream: _pymeta.TokenStream):
        stream.append_bytes_literal(self.type, self.value, self.span)


# @formatter:off
@overload
def lit(value: str, /, *, span: Span | None = None) -> StrLiteral: ...
@overload
def lit(value: bytes | bytearray | memoryview[Any], /, *, span: Span | None = None) -> BytesLiteral: ...
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
        case (bytes(bts) | bytearray(bts) | memoryview(bts), ()) | (None, ("bytes", bts)):
            return BytesLiteral(bytes(bts), BytesLiteral.BYTES, span)
        case (None, ("chr", char)):
            return StrLiteral(str(char), StrLiteral.CHR, span)
        case (None, ("cstr", cstr)):
            return StrLiteral(str(cstr), StrLiteral.CSTR, span)
        case (None, ("byte", SupportsBytes(byte))):
            return BytesLiteral(byte, BytesLiteral.BYTE, span)
        case (None, ("byte", int(byte))):
            return BytesLiteral(byte.to_bytes(signed=True))
        case (None, ("byte", invalid_byte)):
            raise TypeError(f"{invalid_byte!r} of type {type(invalid_byte)} can't be converted into a byte literal")
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
def rust(*args: CoerceToTokens) -> None | Group:
    """Take a list of items, coerce them into ``Tokens``, then add them to the current ``Tokens`` context.

    :return: The last ``Token`` coerced from args if that token is a ``Group``, ``None`` elsewise.
        The intended use case for this is to be used together with a ``with`` statement
        to enter the ``Group`` at the end of the coercion result.
    """
    tokens = Tokens(*args)
    Tokens._current_ctx().extend(tokens)
    if len(tokens) > 0 and isinstance(tokens[-1], Group):
        return tokens[-1]
    return None
