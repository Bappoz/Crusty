int classify(int n) {
    int result = 0;
    switch (n) {
        case 1:
            result = 1;
            break;
        case 2:
            result = 2;
            break;
        default:
            result = -1;
            break;
    }
    return result;
}

int main(void) {
    int total = 0;
    for (int i = 0; i < 3; i = i + 1) {
        total = total + classify(i);
    }
    while (total > 0) {
        total = total - 1;
    }
    if (total == 0) {
        return 0;
    } else {
        return 1;
    }
}
