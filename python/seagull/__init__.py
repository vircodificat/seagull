from .seagull import *
from . import seagull as _ext

Key.__module__ = __name__
Stroke.__module__ = __name__
Outline.__module__ = __name__
Dictionary.__module__ = __name__
JsonDictionary.__module__ = __name__

__doc__ = _ext.__doc__
