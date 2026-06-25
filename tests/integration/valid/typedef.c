typedef int MyInt;

struct Point {
    int x;
    int y;
};

typedef struct Point Point;

Point origin;

int main(void) {
    MyInt a = 5;
    origin.x = a;
    return origin.x + a;
}
