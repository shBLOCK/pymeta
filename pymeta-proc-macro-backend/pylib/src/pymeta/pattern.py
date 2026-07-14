import math
from abc import abstractmethod, ABC
from collections import deque
from collections.abc import Iterable, Callable, Mapping, Sequence, Collection
from typing import Self, Any, final, override, overload

from pymeta import TokensView, Token, Group, Tokens, IntLiteral, FloatLiteral, BoolLiteral, StrLiteral, BytesLiteral, \
    Ident


if not 0:
    raise RuntimeError("WIP")


class MatchResult:
    __slots__ = ("__dict",)

    def __init__(self):
        self.__dict = {}

    def _set__(self, name: str, value: Any):
        self.__dict[name] = value
    
    def _resolve_lazy_results__(self):
        for k, v in self.__dict.items():
            if isinstance(v, _Lazy):
                self.__dict[k] = v.resolve()

    def __getattr__(self, name: str):
        if name in self.__dict:
            return self.__dict[name]
        else:
            raise AttributeError(name)

    def __getitem__(self, name: str, /):
        if name in self.__dict:
            return self.__dict[name]
        else:
            raise KeyError(name)

    def __len__(self):
        return len(self.__dict)

    def __iter__(self) -> Iterable[tuple[str, Any]]:
        yield from self.__dict.items()


class _Lazy[T]:
    def __init__(self, f: Callable[T]):
        self.f = f

    def resolve(self):
        if self.f is None:
            raise RuntimeError("Already resolved")
        f = self.f
        self.f = None
        return f()


class Matcher[R](ABC):
    name: str | None = None
    next: Matcher[Any]

    @abstractmethod
    def _match(self, tokens: TokensView, result: MatchResult | None) -> Iterable[R | _Lazy[R]]:
        ...

    @final
    def match(self, tokens: TokensView, result: MatchResult | None) -> Iterable[R | _Lazy[R]]:
        pos = tokens.pos
        for value in self._match(tokens, result):
            if self.name is not None and result is not None:
                result._set__(self.name, value)

            for _ in self.next.match(tokens, result):
                yield value
        assert tokens.pos == pos
    
    def wrapped[B](self, f: Callable[[R], B]) -> WrappedMatcher[R, B]:
        return WrappedMatcher(self, f)


class DummyMatcher(Matcher[None]):
    def __init__(self, matches: bool = True):
        self.matches = matches
    
    def _match(self, tokens: TokensView, result: MatchResult | None) -> Iterable[None]:
        if self.matches:
            yield None


Matcher.next = DummyMatcher(True)


class TokenMatcher(Matcher[Token]):
    def __init__(self, whitelist: Collection[Token | type[Token]] | Token | type[Token] | None = None, blacklist: Collection[Token | type[Token]] = ()):
        if isinstance(whitelist, Token | type):
            whitelist = (whitelist,)
        self.whitelist: Collection[Token | type[Token]] | None = whitelist
        self.blacklist = blacklist

    @staticmethod
    def _match_token(token: Token, rules: Iterable[Token | type[Token]]) -> bool:
        for rule in rules:
            if isinstance(rule, Token):
                if token == rule:
                    return True
            elif isinstance(rule, type):
                if isinstance(token, rule):
                    return True
            else:
                raise TypeError(rule)
        return False

    def _match(self, tokens: TokensView, result: MatchResult | None) -> Iterable[Token]:
        if not tokens:
            return
        token = tokens[0]
        in_whitelist = self.whitelist is None or TokenMatcher._match_token(token, self.whitelist)
        not_in_blacklist = not TokenMatcher._match_token(token, self.blacklist)
        if in_whitelist and not_in_blacklist:
            tokens.pos += 1
            yield token
            tokens.pos -= 1


class CapturingMatcher(Matcher[TokensView]):
    def __init__(self, inner: Matcher[Any]):
        self.inner = inner

    def _match(self, tokens: TokensView, result: MatchResult | None) -> Iterable[_Lazy[TokensView]]:
        org_pos = tokens.pos
        for _ in self.inner.match(tokens, result):
            pos = tokens.pos
            yield _Lazy(lambda: tokens.referent[org_pos:pos])


class GroupMatcher(Matcher[Group]):
    def __init__(self, delimiter: str, inner: Matcher[Any]):
        self.delimiter = delimiter
        self.inner = inner

    def _match(self, tokens: TokensView, result: MatchResult | None) -> Iterable[Group]:
        if not tokens:
            return
        token = tokens[0]
        if isinstance(token, Group) and token.delimiter == self.delimiter:
            tokens.pos += 1
            for _ in self.inner.match(token.tokens[:], result):
                yield token
            tokens.pos -= 1


class OptionalMatcher[T](Matcher[T | None]):
    def __init__(self, inner: Matcher[T], greedy: bool):
        self.inner = inner
        self.greedy = greedy

    def _match(self, tokens: TokensView, result: T | None) -> Iterable[T | None]:
        for value in self.inner.match(tokens, None):
            yield value
        yield None


class RepeatMatcher(Matcher[Sequence[MatchResult]]):
    def __init__(self, inner: Matcher[Any], n_min: int | None = None, n_max: int | None = None, *,
                 separator: Matcher[Any] = DummyMatcher(True), greedy: bool = True, force_separator: bool = True):
        self.inner = inner
        self.n_min = n_min if n_min is not None else -math.inf
        self.n_max = n_max if n_max is not None else math.inf
        self.separator = separator
        self.greedy = greedy
        self.force_separator = force_separator

    def _match_n(self, tokens: TokensView, results: deque[MatchResult]) -> Iterable[None]:
        n = len(results)
        in_range = self.n_min <= n <= self.n_max
        
        if in_range and not self.greedy:
            yield
        
        if n < self.n_max:
            if not self.force_separator:
                sep_matches = self.separator.match(tokens, None) if n > 0 else (None,)
                for _ in sep_matches:
                    inner_result = MatchResult()
                    results.append(inner_result)
                    for _ in self.inner.match(tokens, inner_result):
                        yield from self._match_n(tokens, results)
                    results.pop()
            else:
                start_pos = tokens.pos
                remaining = tokens[:]
                for end_pos in range(start_pos, tokens.end + 1):
                    remaining.pos = end_pos
                    if any(True for _ in self.separator.match(remaining, None)):
                        break
                # noinspection PyUnboundLocalVariable
                segment = tokens.referent[start_pos:end_pos]
                
                tokens.pos = end_pos
                inner_result = MatchResult()
                results.append(inner_result)
                for _ in self.inner.match(segment, inner_result):
                    if len(segment) == 0:
                        yield from self._match_n(remaining, results)
                results.pop()
                tokens.pos = start_pos

        if in_range and self.greedy:
            yield

    def _match(self, tokens: TokensView, result: MatchResult | None) -> Iterable[_Lazy[Sequence[MatchResult]]]:
        results = []
        lazy_results = _Lazy(lambda: tuple(results))
        for _ in self._match_n(tokens, results):
            yield lazy_results


class OptionsMatcher(Matcher[Any]):
    def __init__(self, *options: Matcher[Any]):
        self.options = options
    
    def _match(self, tokens: TokensView, result: MatchResult | None) -> Iterable[MatchResult]:
        for option in self.options:
            for value in option.match(tokens, None):
                yield value


class EndMatcher(Matcher[None]):
    def _match(self, tokens: TokensView, result: MatchResult | None) -> Iterable[None]:
        if len(tokens) == 0:
            yield None


class WrappedMatcher[A, B](Matcher[B]):
    def __init__(self, inner: Matcher[A], f: Callable[[A], B]):
        self.inner = inner
        self.f = f
    
    def _match(self, tokens: TokensView, result: MatchResult | None) -> Iterable[B]:
        for value in self.inner.match(tokens, result):
            yield self.f(value)


# def matcher_from_python_type(typ) -> Matcher[Any] | None:
#     if typ is int:
#         return TokenMatcher(IntLiteral).wrapped(lambda it: int(it.value))
#     elif typ is float:
#         return TokenMatcher(FloatLiteral).wrapped(lambda it: float(it.value))
#     elif typ is bool:
#         return TokenMatcher(BoolLiteral).wrapped(lambda it: bool(it.value))
#     elif typ is str:
#         return TokenMatcher(StrLiteral).wrapped(lambda it: it.value)
#     elif typ is bytes:
#         return TokenMatcher(BytesLiteral).wrapped(lambda it: it.value)
#     elif typ is None:
#         return TokenMatcher(Ident("None")).wrapped(lambda _: None)
#     elif isinstance(typ, )


class Pattern:
    def __init__(self):
        self._matcher: Matcher[Any]
    
    def match(self, tokens: TokensView | Tokens, *, full: bool = True) -> MatchResult | None:
        if isinstance(tokens, Tokens):
            tokens = tokens[:]
        if not isinstance(tokens, TokensView):
            raise TypeError(tokens)
        
        result = MatchResult()
        for _ in self._matcher.match(tokens, result):
            if len(tokens) == 0 or not full:
                result._resolve_lazy_results__()
                return result
        
        return None
