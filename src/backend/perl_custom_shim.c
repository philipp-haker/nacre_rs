#include <EXTERN.h>
#include <perl.h>

/* Interpreter variables (intrpvar.h). Naming the parameter my_perl makes the
* PL_* macros work in both builds: vTHX->I* with MULTIPLICITY, a global without. */
U8 *emperl_exit_flags_ptr(PerlInterpreter *my_perl) {
    PERL_UNUSED_ARG(my_perl);
    return &PL_exit_flags;
}
U32 *emperl_origalen_ptr(PerlInterpreter *my_perl) {
    PERL_UNUSED_ARG(my_perl);
    return &PL_origalen;
}
signed char *emperl_perl_destruct_level_ptr(PerlInterpreter *my_perl) {
    PERL_UNUSED_ARG(my_perl);
    return &PL_perl_destruct_level;
}
const OP *emperl_restartop(PerlInterpreter *my_perl) {
    PERL_UNUSED_ARG(my_perl);
    return PL_restartop;
}