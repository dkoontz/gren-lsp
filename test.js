function test(a) {
  return a + 1;
}

const b = test(23);

const c = b + 2 + test(99);

// This is the foo class
class Foo {
  Foo(a, b, c) {
    this.a = a;
    this.b = b;
    if (a > b && b > c) {
      this.c = true;
    } else {
      this.c = false;
    }
  }
}

const d = new Foo(3, 4, 1);
