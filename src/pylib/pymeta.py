from abc import ABC, abstractmethod
from collections import deque
from string.templatelib import Template
from typing import SupportsInt, SupportsFloat, final, Collection, Final, SupportsBytes, MutableSequence, overload, Self


@final
class Span:
    # TODO

    @classmethod
    def def_size(cls) -> Span:
        return Span()  # TODO

    @classmethod
    def call_site(cls) -> Span:
        return Span()  # TODO

    @classmethod
    def mixed_site(cls) -> Span:
        return Span()  # TODO

    def __repr__(self):
        return f"{self.__class__.__name__}()"  # TODO


class Token(ABC):
    span: Span

    def __init__(self, span: Span | None = None):
        self.span = span or Span.call_site()

    @abstractmethod
    def __str__(self): ...

    @abstractmethod
    def __repr__(self): ...


type CoerceToTokens = Tokens | Token | Template | int | float | str | bytes

@final
class Tokens(MutableSequence[Token]):
    _CTX_STACK = deque()

    @classmethod
    def _current_ctx(cls) -> Tokens:
        return cls._CTX_STACK[-1]

    def __init__(self, *args: CoerceToTokens):
        def coerce():
            yield  # TODO

        self._tokens = list(coerce())

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
    def __getitem__(self, index: int) -> Token: ...

    @overload
    def __getitem__(self, index: slice) -> Tokens: ...

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

# noinspection PyProtectedMember
Tokens._CTX_STACK.append(Tokens())


@final
class Group(Token):
    PARENTHESIS: Final[str] = "()"
    BRACE: Final[str] = "{}"
    BRACKET: Final[str] = "[]"
    NONE: Final[str] = ""
    DELIMITERS: Final[Collection[str]] = (PARENTHESIS, BRACE, BRACKET, NONE)

    delimiter: str
    tokens: Tokens

    def __init__(self, delimiter: str, tokens: Tokens | None = None, span: Span | None = None):
        super().__init__(span)
        if delimiter not in Group.DELIMITERS:
            raise ValueError(f"invalid group delimiter: \"{delimiter}\"")
        self.delimiter = delimiter
        self.tokens = tokens if tokens is not None else Tokens()

    def __str__(self):
        delim = self.delimiter if self.delimiter != "" else "∅∅"
        return f"{delim[0]} {self.tokens} {delim[1]}"

    def __repr__(self):
        return f"{self.__class__.__name__}(\"{self.delimiter}\", {self.tokens!r}, {self.span!r})"

    def __enter__(self):
        return self.tokens.__enter__()

    def __exit__(self, *_):
        self.tokens.__exit__()


@final
class Ident(Token):
    value: str

    def __init__(self, value: str, span: Span | None = None):
        super().__init__(span)
        self.value = value

    def __str__(self):
        return self.value

    def __repr__(self):
        return f"{self.__class__.__name__}({self.value!r}, {self.span!r})"


@final
class Punct(Token):
    CHARS: Final[Collection[str]] = tuple("=<>!~+-*/%^&|@.,;:#$?'")

    value: str

    def __init__(self, value: str, span: Span | None = None):
        super().__init__(span)
        for c in value:
            if c not in Punct.CHARS:
                raise ValueError(f"invalid punctuation char '{c}'")
        self.value = value

    def __str__(self):
        return self.value

    def __repr__(self):
        return f"{self.__class__.__name__}({self.value!r}, {self.span!r})"


class Literal[T](Token, ABC):
    value: T


@final
class IntLiteral(Literal[int]):
    SUFFIXES: Final[Collection[str]] = tuple(a + b for a in "ui" for b in ("8", "16", "32", "64", "128", "size"))

    suffix: str | None

    def __init__(self, value: SupportsInt, suffix: str | None = None, span: Span | None = None):
        super().__init__(span)
        self.value = int(value)
        if isinstance(suffix, str) and suffix not in IntLiteral.SUFFIXES:
            raise ValueError(f"invalid {self.__class__.__name__} suffix \"{suffix}\"")
        self.suffix = suffix

    def __str__(self):
        return f"{self.value}{self.suffix or ""}"

    def __repr__(self):
        return f"{self.__class__.__name__}({self.value!r}, \"{self.suffix}\", {self.span!r})"

def litint(value: SupportsInt, span: Span | None = None) -> IntLiteral:
    return IntLiteral(value, None, span)

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

    suffix: str | None

    def __init__(self, value: SupportsFloat, suffix: str | None = None, span: Span | None = None):
        super().__init__(span)
        self.value = float(value)
        if isinstance(suffix, str) and suffix not in FloatLiteral.SUFFIXES:
            raise ValueError(f"invalid {self.__class__.__name__} suffix \"{suffix}\"")
        self.suffix = suffix

    def __str__(self):
        return f"{self.value}{self.suffix or ""}"

    def __repr__(self):
        return f"{self.__class__.__name__}({self.value!r}, \"{self.suffix}\", {self.span!r})"

def litfloat(value: SupportsFloat, span: Span | None = None) -> FloatLiteral:
    return FloatLiteral(value, None, span)

def f32(value: SupportsFloat, span: Span | None = None) -> FloatLiteral:
    return FloatLiteral(value, "f32", span)

def f64(value: SupportsFloat, span: Span | None = None) -> FloatLiteral:
    return FloatLiteral(value, "f64", span)


class StrLiteral(Literal[str]):
    CHAR: Final[str] = "char"
    STR: Final[str] = "str"
    CSTR: Final[str] = "cstr"

    type: str

    def __init__(self, value: str, type: str, span: Span | None = None):
        super().__init__(span)
        if type not in (StrLiteral.STR, StrLiteral.CHAR, StrLiteral.CSTR):
            raise ValueError(f"invalid {self.__class__.__name__} type \"{type}\"")
        if type == StrLiteral.CHAR and len(value) != 1:
            raise ValueError(f"\"{value}\" is not a single character")
        self.value = value
        self.type = type

    def __str__(self):
        return repr(self.value)  # TODO: rust format

    def __repr__(self):
        return f"{self.__class__.__name__}({self.value!r}, \"{self.type}\", {self.span!r})"

def litchr(value: str, span: Span | None = None) -> StrLiteral:
    return StrLiteral(value, StrLiteral.CHAR, span)

def litstr(value: str, span: Span | None = None) -> StrLiteral:
    return StrLiteral(value, StrLiteral.STR, span)

def cstr(value: str, span: Span | None = None) -> StrLiteral:
    return StrLiteral(value, StrLiteral.CSTR, span)


class ByteLiteral(Literal[bytes]):
    BYTE: Final[str] = "byte"
    BYTES: Final[str] = "bytes"

    type: str

    def __init__(self, value: SupportsBytes, type: str, span: Span | None = None):
        super().__init__(span)
        value = bytes(value)
        if type not in (ByteLiteral.BYTE, ByteLiteral.BYTES):
            raise ValueError(f"invalid {self.__class__.__name__} type \"{type}\"")
        if type == ByteLiteral.BYTE and len(value) != 1:
            raise ValueError(f"\"{value}\" is not a single byte")
        self.value = value
        self.type = type

    def __str__(self):
        return repr(self.value)  # TODO: rust format

    def __repr__(self):
        return f"{self.__class__.__name__}({self.value!r}, \"{self.type}\", {self.span!r})"

def litbyte(value: bytes, span: Span | None = None) -> ByteLiteral:
    return ByteLiteral(value, ByteLiteral.BYTE, span)

def litbytes(value: bytes, span: Span | None = None) -> ByteLiteral:
    return ByteLiteral(value, ByteLiteral.BYTES, span)


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
