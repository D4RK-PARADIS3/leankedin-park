"""Check the stub against the installed extension without accessing LinkedIn.

Build/install the extension first, then run unittest discovery in this directory.
AST inspection preserves overloads and property setters, which importing a .pyi
as Python would overwrite. Runtime inspection checks what PyO3 actually exposes.

Checks cross-reference three sources of information:
  - the stub describes the contract advertised to IDEs and type checkers;
  - the compiled extension exposes names and signatures actually accessible;
  - the Rust binding source provides the types that Python introspection cannot give.

The imported extension must be rebuilt after any modification of the binding.
Otherwise, tests would compare the current stub against an outdated compiled API.
These tests do not contact LinkedIn and verify neither its API responses,
nor wheel contents, nor IDE autocompletion popups.
"""

import ast
import inspect
from pathlib import Path
import re
from types import ModuleType
import unittest

import leankedin_park as extension


# Anchoring paths on this file allows running tests from another directory.
# They require the source repository, not just the installed package.
STUB = Path(__file__).with_name("leankedin_park.pyi")
SOURCE = Path(__file__).parent / "src" / "lib.rs"


def decorators(function):
    """Make AST decorators comparable without executing the stub."""
    # A set is sufficient: we check for the presence of @property, @staticmethod
    # or @name.setter; their order does not matter in these checks.
    return {ast.unparse(node) for node in function.decorator_list}


def rust_type(raw, owner, argument=False):
    """Translate the binding's small type vocabulary, not arbitrary Rust syntax.

    Fail on a new unfamiliar type so extending the binding requires reviewing
    its Python contract. String collection inputs advertise lists/tuples to
    exclude bare str; outputs are lists.
    """
    # Rust lifetimes describe memory borrows, not Python parameters.
    # For example, Bound<'py, PyDict> becomes Bound<PyDict> here.
    value = re.sub(r"'(?:py|_)\s*,\s*", "", raw.strip())
    value = value.removeprefix("&").removeprefix("'static ")
    # Unfolding recursively handles Option<Vec<String>> just as easily as a simple type,
    # without maintaining a table of all combinations.
    for wrapper in (
        "Option",
        "Vec",
        "PyLnResult",
        "PyResult",
        "Bound",
        "PyRefMut",
        "PyRef",
    ):
        if value.startswith(wrapper + "<") and value.endswith(">"):
            inner = rust_type(value[len(wrapper) + 1 : -1], owner, argument)
            if wrapper == "Option":
                # Rust absence is exposed as None in Python.
                return inner + " | None"
            if wrapper == "Vec":
                # PyO3 accepts lists and tuples on input, but constructs a
                # Python list when returning a Rust Vec.
                if argument and inner == "str":
                    return "list[str] | tuple[str, ...]"
                return ("Sequence[" if argument else "list[") + inner + "]"
            # Result indicates success or an exception, not a Python Result object.
            # Bound/PyRef describe internal handles to PyO3.
            return inner
    # This translation describes type categories. It does not guarantee that a
    # Python int fits within a u8/usize range, nor that a str is a valid URN:
    # those constraints remain validated during runtime invocation.
    primitives = {
        "Self": owner,
        "String": "str",
        "str": "str",
        "f64": "float",
        "usize": "int",
        "u8": "int",
        "i32": "int",
        "i64": "int",
        "u32": "int",
        "u64": "int",
        "()": "None",
        "PyString": "str",
        "PyDict": "dict[str, object]",
        "PathBuf": "str | PathLike[str]",
        "PyAny": "object",
        "bool": "bool",
    }
    if value in primitives:
        return primitives[value]
    # All other supported types are registered Python classes, checked below.
    if value not in StubContractTests.classes:
        raise AssertionError("Unreviewed Rust binding type: " + value)
    return value


def rust_arguments(parameters):
    """Tokenize arguments from Rust signatures present in this binding."""
    # Commas inside nested generic types are not argument separators.
    # A split(',') would break PyRefMut<'py, Self> into two arguments.
    # This scanner does not handle the full Rust grammar: a new signature form
    # may require adapting it and updating corresponding tests.
    depth, start = 0, 0
    for index, character in enumerate(parameters):
        if character == "<":
            depth += 1
        elif character == ">":
            depth -= 1
        elif character == "," and depth == 0:
            yield parameters[start:index]
            start = index + 1
    yield parameters[start:]


def stub_signature(function, constructor=False):
    """Build an inspect.Signature from stub parameters."""
    # Annotations are checked elsewhere: inspect.signature generally does not
    # provide Rust types for extension methods.
    arguments = function.args
    positional = arguments.posonlyargs + arguments.args
    # In the AST, default values correspond to the last positional parameters.
    # Markers are added for required parameters.
    # Parameter.empty means "no default"; None is a real default value.
    defaults = [inspect.Parameter.empty] * (len(positional) - len(arguments.defaults))
    # literal_eval accepts expected literals (None, 0, False...) without eval.
    # A default that becomes an expression requires an explicit check review.
    defaults += [ast.literal_eval(value) for value in arguments.defaults]
    parameters = []
    for index, (argument, default) in enumerate(zip(positional, defaults)):
        if constructor and argument.arg == "self":
            # inspect.signature(Class) describes Class(...), without the instance that
            # the stub names self in __init__. We must compare the same view.
            continue
        # Preserving / and * is essential: matching names is not enough if the
        # stub allows named calls that the extension rejects.
        kind = (
            inspect.Parameter.POSITIONAL_ONLY
            if index < len(arguments.posonlyargs)
            else inspect.Parameter.POSITIONAL_OR_KEYWORD
        )
        parameters.append(inspect.Parameter(argument.arg, kind, default=default))
    for argument, default in zip(arguments.kwonlyargs, arguments.kw_defaults):
        # Here, a missing AST node indicates a required argument;
        # a Constant(None) node indicates an optional argument defaulting to None.
        parameters.append(
            inspect.Parameter(
                argument.arg,
                inspect.Parameter.KEYWORD_ONLY,
                default=inspect.Parameter.empty
                if default is None
                else ast.literal_eval(default),
            )
        )
    return inspect.Signature(parameters)


class StubContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        # Parsing once preserves all overloads and getter/setter pairs.
        # Importing .pyi as a module would overwrite successive functions of the
        # same name and lose this information.
        # feature_version checks syntax against the declared minimum target.
        # This is not execution under Python 3.10 nor a type checker.
        cls.tree = ast.parse(STUB.read_text(encoding="utf-8"), feature_version=(3, 10))
        cls.classes = {
            node.name: node for node in cls.tree.body if isinstance(node, ast.ClassDef)
        }

    def test_exports_and_aliases_match_the_extension(self):
        """Detect both forgotten exports and non-existent names."""
        # __all__ is read from the stub; the extension does not need to define one.
        # We compare against public names actually present in the module.
        exported = next(
            ast.literal_eval(node.value)
            for node in self.tree.body
            if isinstance(node, ast.Assign)
            and any(isinstance(t, ast.Name) and t.id == "__all__" for t in node.targets)
        )
        public_names = {name for name in dir(extension) if not name.startswith("_")}
        # Maturin's wheel wraps the native extension in a Python package.
        # Importing it exposes the child module as well as the public API.
        # Exclude only that exact module, never an arbitrary unexpected export.
        native = getattr(extension, "leankedin_park", None)
        if (
            isinstance(native, ModuleType)
            and native.__name__ == "leankedin_park.leankedin_park"
        ):
            public_names.remove("leankedin_park")
        self.assertEqual(set(exported), public_names)
        aliases = {}
        # A simple assignment like People = PeopleResult describes an alias.
        # The __all__ list does not match this form and is therefore excluded.
        for node in self.tree.body:
            if (
                isinstance(node, ast.Assign)
                and isinstance(node.value, ast.Name)
                and len(node.targets) == 1
                and isinstance(node.targets[0], ast.Name)
            ):
                aliases[node.targets[0].id] = node.value.id
        self.assertEqual(set(exported), set(self.classes) | set(aliases))
        for alias, target in aliases.items():
            # assertIs requires the exact same class object, not just similar classes:
            # this is the behavior promised by a real alias.
            self.assertIs(getattr(extension, alias), getattr(extension, target))

    def test_all_members_and_property_access_match(self):
        """Verify public members and their nature: property or method."""
        for name, node in self.classes.items():
            runtime = getattr(extension, name)
            if issubclass(runtime, Exception):
                # Exceptions inherit standard methods and HttpApiError receives instance
                # attributes when raised from Rust. Their presence cannot be deduced from
                # the class dictionary; this check only verifies their direct base.
                self.assertEqual(
                    runtime.__bases__[0].__name__, ast.unparse(node.bases[0])
                )
                continue
            declared = {
                child.name for child in node.body if isinstance(child, ast.FunctionDef)
            }
            declared |= {
                child.target.id
                for child in node.body
                if isinstance(child, ast.AnnAssign)
            }
            # Python's inherited object methods and PyO3 implementation details
            # need not be repeated; domain members must match in both directions.
            public_runtime = {
                member for member in runtime.__dict__ if not member.startswith("_")
            }
            public_stub = {member for member in declared if not member.startswith("_")}
            with self.subTest(class_name=name):
                # A subtest names the failing class while allowing other classes
                # to continue being checked after a failure.
                self.assertEqual(public_stub, public_runtime)
            for child in node.body:
                if isinstance(child, ast.AnnAssign):
                    # Enum variants must be class constants of the declared enum,
                    # rather than arbitrary writable per-instance attributes.
                    self.assertEqual(
                        ast.unparse(child.annotation), "ClassVar[" + name + "]"
                    )
                    self.assertIsInstance(getattr(runtime, child.target.id), runtime)
                if not isinstance(child, ast.FunctionDef):
                    continue
                tags = decorators(child)
                # getattr_static avoids calling a getter or binding a method.
                # We inspect the descriptor installed by PyO3 itself.
                descriptor = inspect.getattr_static(runtime, child.name)
                if "property" in tags:
                    self.assertTrue(
                        inspect.isdatadescriptor(descriptor), (name, child.name)
                    )
                elif any(tag.endswith(".setter") for tag in tags):
                    self.assertTrue(
                        inspect.isdatadescriptor(descriptor), (name, child.name)
                    )
                    # PyO3 exposes field descriptors rather than Python property objects.
                    # The presence of __set__ is a structural check: it does not prove
                    # that every value will be accepted on assignment.
                    self.assertIsNotNone(descriptor.__set__, (name, child.name))
                elif child.name not in {"__new__", "__init__"}:
                    self.assertFalse(
                        inspect.isdatadescriptor(descriptor), (name, child.name)
                    )
                    self.assertEqual(
                        "staticmethod" in tags,
                        isinstance(descriptor, staticmethod),
                        (name, child.name),
                    )

    def test_names_defaults_and_parameter_kinds_match(self):
        """Compare advertised call patterns with extension signatures."""
        for name, node in self.classes.items():
            runtime = getattr(extension, name)
            for child in node.body:
                if not isinstance(child, ast.FunctionDef):
                    continue
                tags = decorators(child)
                if "property" in tags or any(t.endswith(".setter") for t in tags):
                    continue
                # The dynamic search entry point is checked separately against
                # its explicit allowed keywords; its runtime signature is *args/**kwargs.
                if child.name in {"__new__", "search_people"}:
                    # Non-constructible classes are tested with a dedicated check below;
                    # __new__ has no useful signature here.
                    continue
                constructor = child.name == "__init__"
                target = runtime if constructor else getattr(runtime, child.name)
                try:
                    actual = inspect.signature(target)
                except ValueError:
                    # PyO3 formatting/integer slots expose no text signature.
                    self.assertIn(child.name, {"__repr__", "__str__", "__int__"})
                    # This limited whitelist prevents a business method without
                    # signature from being silently ignored by the test.
                    continue
                with self.subTest(class_name=name, method=child.name):
                    self.assertEqual(stub_signature(child, constructor), actual)

    def test_search_overloads_cover_the_validated_arguments(self):
        """Verify both calling styles for dynamic search."""
        # *args/**kwargs does not tell the IDE which filters are available.
        # The list actually validated by Rust serves as reference for the keyword
        # overload in the stub, instead of duplicating a hardcoded list in this test.
        source = SOURCE.read_text(encoding="utf-8")
        block = re.search(r"const KEYWORDS: &\[&str\] = &\[(.*?)\];", source, re.S)
        allowed = set(re.findall(r'"([a-z_]+)"', block.group(1)))
        methods = [
            node
            for node in self.classes["Linkedin"].body
            if isinstance(node, ast.FunctionDef) and node.name == "search_people"
        ]
        self.assertEqual(len(methods), 2)
        # The order matches the stub: positional builder, then keyword filters.
        # Reorganizing the stub requires adjusting this selection.
        builder, keywords = methods
        self.assertEqual([a.arg for a in builder.args.posonlyargs], ["self", "builder"])
        self.assertEqual(
            ast.unparse(builder.args.posonlyargs[1].annotation),
            "SearchPeople | SearchPeopleBuilder",
        )
        self.assertEqual({a.arg for a in keywords.args.kwonlyargs}, allowed)
        for name in ("SearchPeople", "SearchPeopleBuilder"):
            # Constructors must provide access to the same filters: a new option
            # must not be available in only one style.
            constructor = next(
                n
                for n in self.classes[name].body
                if isinstance(n, ast.FunctionDef) and n.name == "__init__"
            )
            self.assertEqual({a.arg for a in constructor.args.args}, allowed)
        builder_constructor = next(
            n
            for n in self.classes["SearchPeopleBuilder"].body
            if isinstance(n, ast.FunctionDef) and n.name == "__init__"
        )
        types = {
            argument.arg: ast.unparse(argument.annotation)
            for argument in builder_constructor.args.args
        }
        # The kwargs extractor alone accepts explicit None for these defaults.
        # Constructors require int/bool for these fields; the search method
        # explicitly accepts None and applies 0/False. This slight difference is
        # intentional and must not be erased from the stub.
        types.update(offset="int | None", include_private_profiles="bool | None")
        for argument in keywords.args.kwonlyargs:
            self.assertEqual(ast.unparse(argument.annotation), types[argument.arg])

    def test_annotations_match_rust_fields_and_methods(self):
        """Complement runtime introspection with explicit types from the Rust binding."""
        source = SOURCE.read_text(encoding="utf-8")
        checked = 0
        # These regexes target the current formatting: outer blocks closed at line
        # start and public fields on a single line. They do not replace a full Rust
        # parser and must be reviewed if the binding structure changes.
        # Compilation validates Rust separately.
        for owner, body in re.findall(r"pub struct (\w+) \{(.*?)\n\}", source, re.S):
            if owner not in self.classes:
                continue
            getters = {
                n.name: n
                for n in self.classes[owner].body
                if isinstance(n, ast.FunctionDef) and "property" in decorators(n)
            }
            setters = {
                n.name: n
                for n in self.classes[owner].body
                if isinstance(n, ast.FunctionDef)
                and any(d.endswith(".setter") for d in decorators(n))
            }
            for name, field_type in re.findall(r"pub (\w+): (.*?),\n", body):
                self.assertEqual(
                    ast.unparse(getters[name].returns),
                    rust_type(field_type, owner),
                    (owner, name),
                )
                if name in setters:
                    # Getters return a list, while setters accept a sequence:
                    # their annotations may differ.
                    argument = setters[name].args.args[-1]
                    self.assertEqual(
                        ast.unparse(argument.annotation),
                        rust_type(field_type, owner, argument=True),
                        (owner, name),
                    )
                checked += 1
        for owner, body in re.findall(
            r"#\[pymethods\]\nimpl (\w+) \{(.*?)\n\}", source, re.S
        ):
            # Only #[pymethods] blocks are exposed to Python. Other impl blocks
            # contain internal functions that do not belong in the stub.
            declarations = {}
            for node in self.classes[owner].body:
                if isinstance(node, ast.FunctionDef) and not any(
                    d.endswith(".setter") for d in decorators(node)
                ):
                    declarations.setdefault(node.name, []).append(node)
            headers = re.findall(
                r"(?:pub )?fn (\w+)(?:<[^>]*>)?\((.*?)\)\s*->\s*(.*?)\s*\{", body, re.S
            )
            for name, parameters, result in headers:
                # Standard object signatures are inherited by the stub (PYI029).
                if name in {"__repr__", "__str__"}:
                    continue
                name = "__init__" if name == "new" and owner != "Linkedin" else name
                # #[new] constructs the object on the Rust side but corresponds to
                # __init__ in this stub, returning None. Linkedin.new is a public
                # static factory, so its name and return type remain.
                expected_return = (
                    "None" if name == "__init__" else rust_type(result, owner)
                )
                if owner == "Linkedin" and name == "__exit__":
                    expected_return = "Literal[False]"
                arguments = {}
                for piece in rust_arguments(parameters):
                    if ":" not in piece:
                        continue
                    argument_name, argument_type = piece.split(":", 1)
                    argument_name = argument_name.strip()
                    if argument_name in {"py", "slf", "args", "kwargs"}:
                        continue  # Python/GIL objects and dynamic dispatch are internal.
                    arguments[argument_name] = rust_type(
                        argument_type, owner, argument=True
                    )
                for declaration in declarations[name]:
                    # A method can have multiple @overload declarations.
                    # Every return must remain compatible with the binding.
                    self.assertEqual(
                        ast.unparse(declaration.returns), expected_return, (owner, name)
                    )
                    for argument in (
                        declaration.args.args
                        + declaration.args.posonlyargs
                        + declaration.args.kwonlyargs
                    ):
                        if argument.arg in arguments:
                            self.assertEqual(
                                ast.unparse(argument.annotation),
                                arguments[argument.arg],
                                (owner, name, argument.arg),
                            )
                    checked += 1
        # This threshold detects a massive loss of coverage, e.g. after a formatting
        # change. It alone does not guarantee regexes recognized every new declaration.
        self.assertGreater(
            checked, 100
        )  # Catch a parser accidentally matching no source blocks.

    def test_builder_methods_and_facade_defaults_work_offline(self):
        """Reproduce discrepancies found with local extension calls."""
        # Building filters performs no search: no client, cookies or account
        # needed to observe builder methods.
        builder = extension.SearchPeopleBuilder(limit=1, offset=2)
        self.assertEqual(builder.limit, 1)
        self.assertEqual(builder.offset, 2)
        builder.limit = 0
        builder.offset = 0
        self.assertEqual((builder.limit, builder.offset), (0, 0))
        builder.limit = None
        self.assertIsNone(builder.limit)
        defaults = extension.SearchPeopleBuilder()
        self.assertEqual((defaults.limit, defaults.offset), (None, 0))
        search = extension.SearchPeople().limit(5).offset(2)
        self.assertEqual((search.build().limit, search.build().offset), (5, 2))
        self.assertIsNone(search.limit().build().limit)
        self.assertIsNone(search.build().limit)  # Fluent calls mutate in-place.
        # This tuple concretely confirms accepted collection inputs;
        # the getter confirms output remains a Python list.
        self.assertEqual(
            extension.SearchPeople(
                current_company=("12345", "67890")
            ).builder.current_company,
            ["12345", "67890"],
        )

    def test_builder_pagination_rejects_invalid_assignments(self):
        builder = extension.SearchPeopleBuilder(limit=5, offset=2)
        for name, value, error in (
            ("limit", -1, OverflowError),
            ("offset", -1, OverflowError),
            ("limit", "5", TypeError),
            ("offset", "2", TypeError),
            ("offset", None, TypeError),
            ("limit", 1 << 128, OverflowError),
            ("offset", 1 << 128, OverflowError),
        ):
            with self.subTest(name=name, value=value), self.assertRaises(error):
                setattr(builder, name, value)
            self.assertEqual((builder.limit, builder.offset), (5, 2))

    def test_enum_hashability_matches_stub(self):
        for name, node in self.classes.items():
            variants = [
                n
                for n in node.body
                if isinstance(n, ast.AnnAssign)
                and ast.unparse(n.annotation) == "ClassVar[" + name + "]"
            ]
            if not variants:
                continue
            runtime = getattr(extension, name)
            with self.subTest(enum=name):
                declaration = next(
                    n
                    for n in node.body
                    if isinstance(n, ast.Assign)
                    and any(
                        isinstance(t, ast.Name) and t.id == "__hash__"
                        for t in n.targets
                    )
                )
                self.assertIsNone(ast.literal_eval(declaration.value))
                self.assertIsNone(runtime.__hash__)
                with self.assertRaises(TypeError):
                    hash(getattr(runtime, variants[0].target.id))

    def test_company_ids_and_string_collection_inputs(self):
        ids = ["12345", "67890"]
        url = (
            "https://www.linkedin.com/search/results/people/"
            "?currentCompany=%5B%2212345%22%2C%2267890%22%5D"
        )
        for factory in (extension.SearchPeople, extension.SearchPeopleBuilder):
            for values in (ids, tuple(ids)):
                result = factory(current_company=values)
                builder = (
                    result.builder if factory is extension.SearchPeople else result
                )
                self.assertEqual(builder.current_company, ids)
            with self.assertRaises(TypeError):
                factory(current_company="12345")
            result = factory.from_url(url)
            builder = result.builder if factory is extension.SearchPeople else result
            self.assertEqual(builder.current_company, ids)
        for method in ("companies", "current_company"):
            for values in (ids, tuple(ids)):
                result = getattr(extension.SearchPeople(), method)(values)
                self.assertEqual(result.builder.current_company, ids)
            with self.assertRaises(TypeError):
                getattr(extension.SearchPeople(), method)("12345")
        builder = extension.SearchPeopleBuilder()
        builder.current_company = tuple(ids)
        self.assertEqual(builder.current_company, ids)
        with self.assertRaises(TypeError):
            builder.current_company = "12345"

    def test_context_manager_never_suppresses_exceptions(self):
        config = extension.ClientConfig.from_cookies("test-li-at", "test-jsessionid")
        client = extension.Linkedin.with_config(config)
        # Construction uses supplied fake cookies; no requests are made.
        self.assertIs(client.__exit__(ValueError, ValueError("test"), None), False)
        self.assertIs(client.__exit__(), False)

    def test_classes_without_constructors_are_documented_as_non_constructible(self):
        """Ensure the stub does not lead users to believe result objects can be instantiated."""
        for name, node in self.classes.items():
            runtime = getattr(extension, name)
            if issubclass(runtime, Exception) or name in {
                "ClientConfig",
                "SearchPeople",
                "SearchPeopleBuilder",
            }:
                # These classes have a public constructor. Others are supplied
                # by the extension, factories, or enum variants.
                continue
            declaration = next(
                child
                for child in node.body
                if isinstance(child, ast.FunctionDef) and child.name == "__new__"
            )
            self.assertEqual(ast.unparse(declaration.returns), "NoReturn")
            # NoReturn describes a call that always raises an exception.
            # The call below specifically checks the zero-argument case;
            # it does not claim to test all argument combinations.
            with self.subTest(class_name=name), self.assertRaises(TypeError):
                runtime()


if __name__ == "__main__":
    unittest.main()
