from typing import Iterable, Sequence, TypeAlias

Point: TypeAlias = tuple[float, float]
RGB: TypeAlias = tuple[float, float, float]
Annotation: TypeAlias = tuple[float, float, str]
SeriesData: TypeAlias = Iterable[float] | Iterable[Point]
MultiSeriesData: TypeAlias = Iterable[SeriesData]

class Line:
    def __init__(
        self,
        data: SeriesData | MultiSeriesData,
        *,
        x: tuple[float, float] | None = ...,
        y: tuple[float, float] | None = ...,
        color: RGB | Sequence[RGB] | None = ...,
        width: float | Sequence[float] | None = ...,
        annotations: Sequence[Annotation] | None = ...,
        title: str | None = ...,
        x_label: str | None = ...,
        y_label: str | None = ...,
        grid: bool = ...,
    ) -> None: ...
    def show(self) -> None: ...
    def save(self, path: str) -> None: ...

class Bar:
    def __init__(
        self,
        data: SeriesData | MultiSeriesData,
        *,
        x: tuple[float, float] | None = ...,
        y: tuple[float, float] | None = ...,
        color: RGB | Sequence[RGB] | None = ...,
        width: float | Sequence[float] | None = ...,
        annotations: Sequence[Annotation] | None = ...,
        title: str | None = ...,
        x_label: str | None = ...,
        y_label: str | None = ...,
        grid: bool = ...,
    ) -> None: ...
    def show(self) -> None: ...
    def save(self, path: str) -> None: ...

class Scatter:
    def __init__(
        self,
        data: SeriesData | MultiSeriesData,
        *,
        x: tuple[float, float] | None = ...,
        y: tuple[float, float] | None = ...,
        color: RGB | Sequence[RGB] | None = ...,
        size: float | Sequence[float] | None = ...,
        marker: str | Sequence[str] | None = ...,
        annotations: Sequence[Annotation] | None = ...,
        title: str | None = ...,
        x_label: str | None = ...,
        y_label: str | None = ...,
        grid: bool = ...,
    ) -> None: ...
    def show(self) -> None: ...
    def save(self, path: str) -> None: ...

def show() -> None: ...
def demo_line() -> None: ...
def demo_scatter() -> None: ...
